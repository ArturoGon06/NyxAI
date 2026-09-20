use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    domain::{conversation::ConversationMessage, settings::GenerationSettings},
    providers::{
        EmbeddingProvider, ModelProvider, ProviderError, ProviderModel, ProviderStream,
        StreamingChunk, TextGenerationProvider,
    },
};

/// Implements the local OpenAI-compatible HTTP protocol. NyxAI always uses a
/// configured private base URL; this adapter never selects a cloud endpoint.
#[derive(Clone)]
pub struct OpenAICompatibleProvider {
    client: Client,
}

impl OpenAICompatibleProvider {
    pub fn new(api_key: Option<String>) -> anyhow::Result<Self> {
        let mut headers = header::HeaderMap::new();
        if let Some(api_key) = api_key.filter(|key| !key.trim().is_empty()) {
            let value =
                header::HeaderValue::from_str(&format!("Bearer {api_key}")).map_err(|_| {
                    anyhow::anyhow!("OPENAI_COMPATIBLE_API_KEY contains invalid header characters")
                })?;
            headers.insert(header::AUTHORIZATION, value);
        }
        Ok(Self {
            client: Client::builder()
                .default_headers(headers)
                .connect_timeout(Duration::from_secs(8))
                .timeout(Duration::from_secs(60 * 60))
                .build()?,
        })
    }
}

#[async_trait]
impl ModelProvider for OpenAICompatibleProvider {
    async fn list_models(&self, base_url: &str) -> Result<Vec<ProviderModel>, ProviderError> {
        let response = self
            .client
            .get(endpoint(base_url, "models")?)
            .send()
            .await
            .map_err(network_error)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(ProviderError::UnsupportedCapability("model discovery"));
        }
        let body: OpenAIModelsResponse = ensure_success(response)
            .await?
            .json()
            .await
            .map_err(invalid_response)?;
        let mut models = body
            .data
            .into_iter()
            .filter_map(|model| {
                let name = model.id.trim();
                (!name.is_empty()).then(|| ProviderModel {
                    name: name.to_owned(),
                    size_bytes: None,
                    parameter_size: None,
                })
            })
            .collect::<Vec<_>>();
        models.sort_by(|left, right| left.name.cmp(&right.name));
        models.dedup_by(|left, right| left.name == right.name);
        Ok(models)
    }
}

#[async_trait]
impl TextGenerationProvider for OpenAICompatibleProvider {
    async fn stream_chat(
        &self,
        base_url: &str,
        model: &str,
        messages: Vec<ConversationMessage>,
        generation: GenerationSettings,
    ) -> Result<ProviderStream, ProviderError> {
        let request = OpenAIChatRequest {
            model,
            messages,
            stream: true,
            temperature: generation.temperature,
            max_tokens: generation.max_response_length,
        };
        let response = self
            .client
            .post(endpoint(base_url, "chat/completions")?)
            .json(&request)
            .send()
            .await
            .map_err(network_error)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(ProviderError::ModelUnavailable);
        }
        let response = ensure_success(response).await?;
        let mut bytes = response.bytes_stream();
        let stream = async_stream::stream! {
            let mut buffered = Vec::new();
            while let Some(next) = bytes.next().await {
                match next {
                    Ok(chunk) => {
                        buffered.extend_from_slice(&chunk);
                        while let Some(newline) = buffered.iter().position(|byte| *byte == b'\n') {
                            let line = buffered.drain(..=newline).collect::<Vec<_>>();
                            if let Some(result) = parse_sse_line(&line) {
                                match result {
                                    Ok(chunk) => yield Ok(chunk),
                                    Err(error) => { yield Err(error); return; }
                                }
                            }
                        }
                    }
                    Err(error) => { yield Err(network_error(error)); return; }
                }
            }
            if let Some(result) = parse_sse_line(&buffered) { yield result; }
        };
        Ok(Box::pin(stream))
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAICompatibleProvider {
    async fn embed(
        &self,
        base_url: &str,
        model: &str,
        input: &str,
    ) -> Result<Vec<f32>, ProviderError> {
        let response = self
            .client
            .post(endpoint(base_url, "embeddings")?)
            .json(&OpenAIEmbeddingRequest { model, input })
            .send()
            .await
            .map_err(network_error)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(ProviderError::UnsupportedCapability("embeddings"));
        }
        let body: OpenAIEmbeddingResponse = ensure_success(response)
            .await?
            .json()
            .await
            .map_err(invalid_response)?;
        let vector = body
            .data
            .into_iter()
            .next()
            .map(|entry| entry.embedding)
            .ok_or_else(|| {
                ProviderError::InvalidResponse("the embedding response was empty".to_owned())
            })?;
        if vector.is_empty() || !vector.iter().all(|value| value.is_finite()) {
            return Err(ProviderError::InvalidResponse(
                "the embedding response contained an invalid vector".to_owned(),
            ));
        }
        Ok(vector)
    }
}

fn endpoint(base_url: &str, path: &str) -> Result<Url, ProviderError> {
    let mut base = Url::parse(base_url.trim()).map_err(|_| ProviderError::InvalidUrl)?;
    if !matches!(base.scheme(), "http" | "https") || base.host_str().is_none() {
        return Err(ProviderError::InvalidUrl);
    }
    base.set_query(None);
    base.set_fragment(None);
    if !base.path().ends_with('/') {
        base.set_path(&format!("{}/", base.path()));
    }
    base.join(path).map_err(|_| ProviderError::InvalidUrl)
}

fn network_error(error: reqwest::Error) -> ProviderError {
    if error.is_timeout() {
        ProviderError::TimedOut(error)
    } else {
        ProviderError::Unreachable(error)
    }
}

fn invalid_response(error: reqwest::Error) -> ProviderError {
    ProviderError::InvalidResponse(error.to_string())
}

async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response, ProviderError> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    // Discard arbitrary server response bodies so they cannot be exposed in a
    // normal user error or logged alongside configuration secrets.
    if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
        return Err(ProviderError::AuthenticationFailed);
    }
    Err(ProviderError::Rejected {
        status: status.as_u16(),
    })
}

fn parse_sse_line(line: &[u8]) -> Option<Result<StreamingChunk, ProviderError>> {
    let line = std::str::from_utf8(line).ok()?.trim();
    let data = line.strip_prefix("data:")?.trim();
    if data == "[DONE]" {
        return Some(Ok(StreamingChunk {
            content: String::new(),
            done: true,
        }));
    }
    let response: OpenAIStreamResponse = match serde_json::from_str(data) {
        Ok(response) => response,
        Err(error) => return Some(Err(ProviderError::InvalidResponse(error.to_string()))),
    };
    if response.error.and_then(|error| error.message).is_some() {
        // An arbitrary upstream error string can contain implementation
        // details. Keep the browser-facing error generic and safe.
        return Some(Err(ProviderError::GenerationFailed(
            "the inference backend reported a generation error".to_owned(),
        )));
    }
    let choice = response.choices.into_iter().next();
    Some(Ok(StreamingChunk {
        content: choice
            .as_ref()
            .and_then(|choice| choice.delta.as_ref())
            .and_then(|delta| delta.content.clone())
            .unwrap_or_default(),
        done: choice.and_then(|choice| choice.finish_reason).is_some(),
    }))
}

#[derive(Deserialize)]
struct OpenAIModelsResponse {
    #[serde(default)]
    data: Vec<OpenAIModel>,
}
#[derive(Deserialize)]
struct OpenAIModel {
    id: String,
}
#[derive(Serialize)]
struct OpenAIChatRequest<'a> {
    model: &'a str,
    messages: Vec<ConversationMessage>,
    stream: bool,
    temperature: f32,
    max_tokens: u32,
}
#[derive(Serialize)]
struct OpenAIEmbeddingRequest<'a> {
    model: &'a str,
    input: &'a str,
}
#[derive(Deserialize)]
struct OpenAIEmbeddingResponse {
    #[serde(default)]
    data: Vec<OpenAIEmbeddingData>,
}
#[derive(Deserialize)]
struct OpenAIEmbeddingData {
    #[serde(default)]
    embedding: Vec<f32>,
}
#[derive(Deserialize)]
struct OpenAIStreamResponse {
    #[serde(default)]
    choices: Vec<OpenAIStreamChoice>,
    error: Option<OpenAIError>,
}
#[derive(Deserialize)]
struct OpenAIStreamChoice {
    delta: Option<OpenAIStreamDelta>,
    finish_reason: Option<serde_json::Value>,
}
#[derive(Deserialize)]
struct OpenAIStreamDelta {
    content: Option<String>,
}
#[derive(Deserialize)]
struct OpenAIError {
    message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openai_compatible_streaming_and_done_marker() {
        let chunk = parse_sse_line(
            b"data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n",
        )
        .expect("event")
        .expect("parse");
        assert_eq!(chunk.content, "Hello");
        assert!(!chunk.done);
        assert!(
            parse_sse_line(b"data: [DONE]\n")
                .expect("event")
                .expect("parse")
                .done
        );
    }

    #[test]
    fn joins_v1_base_urls_without_cloud_assumptions() {
        assert_eq!(
            endpoint("http://127.0.0.1:8080/v1", "models")
                .expect("url")
                .as_str(),
            "http://127.0.0.1:8080/v1/models"
        );
    }

    #[test]
    fn normalizes_openai_compatible_model_discovery() {
        let response: OpenAIModelsResponse =
            serde_json::from_str(r#"{"data":[{"id":"local-qwen"},{"id":"local-embed"}]}"#)
                .expect("fixture must deserialize");
        assert_eq!(response.data[0].id, "local-qwen");
        assert_eq!(response.data[1].id, "local-embed");
    }

    #[test]
    fn invalid_api_key_value_does_not_echo_the_secret() {
        let secret = "do-not-repeat\r\nthis";
        let error = match OpenAICompatibleProvider::new(Some(secret.to_owned())) {
            Ok(_) => panic!("newlines are invalid header values"),
            Err(error) => error,
        };
        assert!(!error.to_string().contains(secret));
    }
}
