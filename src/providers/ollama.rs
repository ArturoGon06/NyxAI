use std::pin::Pin;

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    domain::{conversation::ConversationMessage, settings::GenerationSettings},
    providers::{
        EmbeddingProvider, ModelLifecycleProvider, ModelProvider, ProviderError, ProviderModel,
        ProviderStream, StreamingChunk, TextGenerationProvider,
    },
};

#[derive(Clone)]
pub struct OllamaProvider {
    client: Client,
}

impl OllamaProvider {
    pub fn new() -> anyhow::Result<Self> {
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(8))
            // A stream can legitimately take a long time. Cancellation drops
            // the response, while this protects against permanently stuck I/O.
            .timeout(std::time::Duration::from_secs(60 * 60))
            .build()?;

        Ok(Self { client })
    }

    pub async fn list_models(&self, base_url: &str) -> Result<Vec<ProviderModel>, ProviderError> {
        let endpoint = api_endpoint(base_url, "api/tags")?;
        let response = self
            .client
            .get(endpoint)
            .send()
            .await
            .map_err(network_error)?;
        let response = ensure_success(response).await?;
        let body: OllamaTagsResponse = response
            .json()
            .await
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;

        let mut models = body
            .models
            .into_iter()
            .filter(|model| !model.name.trim().is_empty())
            .map(|model| ProviderModel {
                name: model.name,
                size_bytes: model.size,
                parameter_size: model.details.and_then(|details| details.parameter_size),
            })
            .collect::<Vec<_>>();
        models.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(models)
    }

    pub async fn stream_chat(
        &self,
        base_url: &str,
        model: &str,
        messages: Vec<ConversationMessage>,
        generation: GenerationSettings,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamingChunk, ProviderError>> + Send>>,
        ProviderError,
    > {
        let endpoint = api_endpoint(base_url, "api/chat")?;
        let request = OllamaChatRequest {
            model,
            messages,
            stream: true,
            options: OllamaGenerationOptions {
                temperature: generation.temperature,
                num_ctx: generation.context_length,
                num_predict: generation.max_response_length,
            },
        };
        let response = self
            .client
            .post(endpoint)
            .json(&request)
            .send()
            .await
            .map_err(network_error)?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(ProviderError::ModelUnavailable);
        }

        let response = ensure_success(response).await?;
        let mut bytes = response.bytes_stream();

        // Ollama streams newline-delimited JSON. Buffer bytes, rather than
        // strings, so a UTF-8 character split across network chunks is intact.
        let stream = async_stream::stream! {
            let mut buffered = Vec::new();

            while let Some(next) = bytes.next().await {
                match next {
                    Ok(chunk) => {
                        buffered.extend_from_slice(&chunk);
                        while let Some(newline) = buffered.iter().position(|byte| *byte == b'\n') {
                            let line = buffered.drain(..=newline).collect::<Vec<_>>();
                            if let Some(result) = parse_stream_line(&line) {
                                match result {
                                    Ok(chunk) => yield Ok(chunk),
                                    Err(error) => {
                                        yield Err(error);
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    Err(error) => {
                        yield Err(network_error(error));
                        return;
                    }
                }
            }

            if let Some(result) = parse_stream_line(&buffered) {
                yield result;
            }
        };

        Ok(Box::pin(stream))
    }

    /// Ollama's local embedding endpoint is intentionally kept beside its
    /// chat implementation so memory retrieval never owns a second HTTP
    /// client or learns Ollama URL details.
    pub async fn embed(
        &self,
        base_url: &str,
        model: &str,
        input: &str,
    ) -> Result<Vec<f32>, ProviderError> {
        let endpoint = api_endpoint(base_url, "api/embed")?;
        let response = self
            .client
            .post(endpoint)
            .json(&OllamaEmbedRequest { model, input })
            .send()
            .await
            .map_err(network_error)?;
        if response.status() == StatusCode::NOT_FOUND {
            return self.embed_legacy(base_url, model, input).await;
        }
        let response = ensure_success(response).await?;
        let body: OllamaEmbedResponse = response
            .json()
            .await
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        let vector = body.embeddings.into_iter().next().ok_or_else(|| {
            ProviderError::InvalidResponse("the embedding response was empty".to_owned())
        })?;
        validate_embedding_vector(vector)
    }

    /// Older local Ollama installations exposed `/api/embeddings` instead of
    /// `/api/embed`. This bounded compatibility retry keeps the provider
    /// abstraction stable without changing memory callers.
    async fn embed_legacy(
        &self,
        base_url: &str,
        model: &str,
        input: &str,
    ) -> Result<Vec<f32>, ProviderError> {
        let endpoint = api_endpoint(base_url, "api/embeddings")?;
        let response = self
            .client
            .post(endpoint)
            .json(&OllamaLegacyEmbedRequest {
                model,
                prompt: input,
            })
            .send()
            .await
            .map_err(network_error)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(ProviderError::ModelUnavailable);
        }
        let body: OllamaLegacyEmbedResponse = ensure_success(response)
            .await?
            .json()
            .await
            .map_err(|error| ProviderError::InvalidResponse(error.to_string()))?;
        validate_embedding_vector(body.embedding)
    }

    /// Ollama releases a loaded model when a generate request sets keep_alive
    /// to zero. This is deliberately best-effort lifecycle support.
    pub async fn unload_model(&self, base_url: &str, model: &str) -> Result<(), ProviderError> {
        let endpoint = api_endpoint(base_url, "api/generate")?;
        let response = self
            .client
            .post(endpoint)
            .json(&OllamaUnloadRequest {
                model,
                keep_alive: 0,
            })
            .send()
            .await
            .map_err(network_error)?;
        ensure_success(response).await?;
        Ok(())
    }

    pub async fn preload_model(&self, base_url: &str, model: &str) -> Result<(), ProviderError> {
        let endpoint = api_endpoint(base_url, "api/generate")?;
        let response = self
            .client
            .post(endpoint)
            .json(&OllamaPreloadRequest {
                model,
                keep_alive: "5m",
            })
            .send()
            .await
            .map_err(network_error)?;
        ensure_success(response).await?;
        Ok(())
    }
}

#[async_trait]
impl ModelProvider for OllamaProvider {
    async fn list_models(&self, base_url: &str) -> Result<Vec<ProviderModel>, ProviderError> {
        OllamaProvider::list_models(self, base_url).await
    }
}

#[async_trait]
impl TextGenerationProvider for OllamaProvider {
    async fn stream_chat(
        &self,
        base_url: &str,
        model: &str,
        messages: Vec<ConversationMessage>,
        generation: GenerationSettings,
    ) -> Result<ProviderStream, ProviderError> {
        OllamaProvider::stream_chat(self, base_url, model, messages, generation).await
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn embed(
        &self,
        base_url: &str,
        model: &str,
        input: &str,
    ) -> Result<Vec<f32>, ProviderError> {
        OllamaProvider::embed(self, base_url, model, input).await
    }
}

#[async_trait]
impl ModelLifecycleProvider for OllamaProvider {
    async fn unload_model(&self, base_url: &str, model: &str) -> Result<(), ProviderError> {
        OllamaProvider::unload_model(self, base_url, model).await
    }

    async fn preload_model(&self, base_url: &str, model: &str) -> Result<(), ProviderError> {
        OllamaProvider::preload_model(self, base_url, model).await
    }
}

fn api_endpoint(base_url: &str, path: &str) -> Result<Url, ProviderError> {
    let mut base = Url::parse(base_url.trim()).map_err(|_| ProviderError::InvalidUrl)?;
    if !matches!(base.scheme(), "http" | "https") || base.host_str().is_none() {
        return Err(ProviderError::InvalidUrl);
    }

    base.set_query(None);
    base.set_fragment(None);
    if !base.path().ends_with('/') {
        let path = format!("{}/", base.path());
        base.set_path(&path);
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

async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response, ProviderError> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status().as_u16();
    // Do not retain arbitrary backend bodies in provider errors. They may be
    // noisy, private, or include implementation details unsuitable for logs.
    let _ = response.bytes().await;
    Err(ProviderError::Rejected { status })
}

fn validate_embedding_vector(vector: Vec<f32>) -> Result<Vec<f32>, ProviderError> {
    if vector.is_empty() || !vector.iter().all(|value| value.is_finite()) {
        return Err(ProviderError::InvalidResponse(
            "the embedding response contained an invalid vector".to_owned(),
        ));
    }
    Ok(vector)
}

fn parse_stream_line(line: &[u8]) -> Option<Result<StreamingChunk, ProviderError>> {
    let line = line
        .iter()
        .copied()
        .filter(|byte| *byte != b'\r' && *byte != b'\n')
        .collect::<Vec<_>>();
    if line.is_empty() {
        return None;
    }

    let response: OllamaChatResponse = match serde_json::from_slice(&line) {
        Ok(response) => response,
        Err(error) => return Some(Err(ProviderError::InvalidResponse(error.to_string()))),
    };

    if let Some(error) = response.error {
        return Some(Err(ProviderError::GenerationFailed(error)));
    }

    Some(Ok(StreamingChunk {
        content: response
            .message
            .map(|message| message.content)
            .unwrap_or_default(),
        done: response.done,
    }))
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
    size: Option<u64>,
    details: Option<OllamaModelDetails>,
}

#[derive(Deserialize)]
struct OllamaModelDetails {
    parameter_size: Option<String>,
}

#[derive(Serialize)]
struct OllamaChatRequest<'a> {
    model: &'a str,
    messages: Vec<ConversationMessage>,
    stream: bool,
    options: OllamaGenerationOptions,
}

#[derive(Serialize)]
struct OllamaEmbedRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize)]
struct OllamaEmbedResponse {
    #[serde(default)]
    embeddings: Vec<Vec<f32>>,
}

#[derive(Serialize)]
struct OllamaLegacyEmbedRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Deserialize)]
struct OllamaLegacyEmbedResponse {
    #[serde(default)]
    embedding: Vec<f32>,
}

#[derive(Serialize)]
struct OllamaGenerationOptions {
    temperature: f32,
    num_ctx: u32,
    num_predict: u32,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: Option<OllamaChatMessage>,
    #[serde(default)]
    done: bool,
    error: Option<String>,
}

#[derive(Deserialize)]
struct OllamaChatMessage {
    #[serde(default)]
    content: String,
}

#[derive(Serialize)]
struct OllamaUnloadRequest<'a> {
    model: &'a str,
    keep_alive: u8,
}
#[derive(Serialize)]
struct OllamaPreloadRequest<'a> {
    model: &'a str,
    keep_alive: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_response_is_normalized_to_models() {
        let response: OllamaTagsResponse = serde_json::from_str(
            r#"{"models":[{"name":"qwen:latest","size":123,"details":{"parameter_size":"4B"}}]}"#,
        )
        .expect("fixture must deserialize");

        let model = response.models.into_iter().next().expect("model expected");
        assert_eq!(model.name, "qwen:latest");
        assert_eq!(
            model.details.and_then(|details| details.parameter_size),
            Some("4B".to_owned())
        );
    }

    #[test]
    fn preserves_stream_content_and_completion_marker() {
        let chunk = parse_stream_line(
            br#"{"message":{"role":"assistant","content":"Hello"},"done":false}"#,
        )
        .expect("line should be present")
        .expect("line should parse");
        assert_eq!(chunk.content, "Hello");
        assert!(!chunk.done);
    }

    #[test]
    fn builds_an_api_endpoint_from_a_base_url() {
        let endpoint = api_endpoint("http://ollama.local:11434/", "api/tags").expect("valid URL");
        assert_eq!(endpoint.as_str(), "http://ollama.local:11434/api/tags");
    }

    #[test]
    fn generation_settings_are_sent_as_ollama_options() {
        let request = OllamaChatRequest {
            model: "test-model",
            messages: vec![ConversationMessage {
                role: crate::domain::conversation::MessageRole::User,
                content: "Hello".to_owned(),
            }],
            stream: true,
            options: OllamaGenerationOptions {
                temperature: 0.65,
                num_ctx: 8_192,
                num_predict: 768,
            },
        };

        let request = serde_json::to_value(request).expect("request must serialize");
        assert_eq!(
            request["options"]["temperature"].as_f64(),
            Some(f64::from(0.65_f32))
        );
        assert_eq!(request["options"]["num_ctx"], 8_192);
        assert_eq!(request["options"]["num_predict"], 768);
    }
}
