use std::pin::Pin;

use futures_util::{Stream, StreamExt};
use serde::Serialize;

use crate::{
    domain::{
        conversation::ConversationMessage,
        settings::{GenerationSettings, ProviderKind},
    },
    providers::ollama::OllamaProvider,
};

#[derive(Clone)]
pub struct ProviderService {
    ollama: OllamaProvider,
}

impl ProviderService {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            ollama: OllamaProvider::new()?,
        })
    }

    pub fn capabilities(&self, provider: &ProviderKind) -> ProviderCapabilities {
        match provider {
            ProviderKind::Ollama => ProviderCapabilities {
                connection_testing: true,
                model_listing: true,
                chat_generation: true,
                streaming_generation: true,
                generation_cancellation: true,
                basic_generation_settings: true,
                embeddings: true,
            },
        }
    }

    /// The provider-facing embedding abstraction. Semantic-memory callers use
    /// this service rather than Ollama HTTP endpoints directly.
    pub async fn embed(
        &self,
        provider: &ProviderKind,
        base_url: &str,
        model: &str,
        input: &str,
    ) -> Result<Vec<f32>, ProviderError> {
        match provider {
            ProviderKind::Ollama => self.ollama.embed(base_url, model, input).await,
        }
    }

    pub async fn test_connection(
        &self,
        provider: &ProviderKind,
        base_url: &str,
    ) -> Result<ConnectionReport, ProviderError> {
        let models = self.list_models(provider, base_url).await?;
        Ok(ConnectionReport {
            connected: true,
            model_count: models.len(),
        })
    }

    pub async fn list_models(
        &self,
        provider: &ProviderKind,
        base_url: &str,
    ) -> Result<Vec<ProviderModel>, ProviderError> {
        match provider {
            ProviderKind::Ollama => self.ollama.list_models(base_url).await,
        }
    }

    pub async fn stream_chat(
        &self,
        provider: &ProviderKind,
        base_url: &str,
        model: &str,
        messages: Vec<ConversationMessage>,
        generation: GenerationSettings,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamingChunk, ProviderError>> + Send>>,
        ProviderError,
    > {
        match provider {
            ProviderKind::Ollama => {
                self.ollama
                    .stream_chat(base_url, model, messages, generation)
                    .await
            }
        }
    }

    /// A bounded, non-persistent text generation primitive for features such
    /// as the Character Creator. It still travels through the provider layer
    /// and therefore does not couple callers to Ollama's HTTP API.
    pub async fn generate_text(
        &self,
        provider: &ProviderKind,
        base_url: &str,
        model: &str,
        messages: Vec<ConversationMessage>,
        generation: GenerationSettings,
    ) -> Result<String, ProviderError> {
        let mut stream = self
            .stream_chat(provider, base_url, model, messages, generation)
            .await?;
        let mut content = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            content.push_str(&chunk.content);
            if chunk.done {
                break;
            }
        }

        if content.trim().is_empty() {
            return Err(ProviderError::GenerationFailed(
                "the model returned an empty response".to_owned(),
            ));
        }
        Ok(content)
    }

    pub async fn unload_model(
        &self,
        provider: &ProviderKind,
        base_url: &str,
        model: &str,
    ) -> Result<(), ProviderError> {
        match provider {
            ProviderKind::Ollama => self.ollama.unload_model(base_url, model).await,
        }
    }

    pub async fn preload_model(
        &self,
        provider: &ProviderKind,
        base_url: &str,
        model: &str,
    ) -> Result<(), ProviderError> {
        match provider {
            ProviderKind::Ollama => self.ollama.preload_model(base_url, model).await,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderCapabilities {
    pub connection_testing: bool,
    pub model_listing: bool,
    pub chat_generation: bool,
    pub streaming_generation: bool,
    pub generation_cancellation: bool,
    pub basic_generation_settings: bool,
    pub embeddings: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionReport {
    pub connected: bool,
    pub model_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderModel {
    pub name: String,
    pub size_bytes: Option<u64>,
    pub parameter_size: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StreamingChunk {
    pub content: String,
    pub done: bool,
}

#[derive(Debug)]
pub enum ProviderError {
    InvalidUrl,
    Unreachable(reqwest::Error),
    TimedOut(reqwest::Error),
    Rejected { status: u16, detail: String },
    ModelUnavailable,
    InvalidResponse(String),
    GenerationFailed(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrl => write!(formatter, "invalid Ollama base URL"),
            Self::Unreachable(error) => write!(formatter, "Ollama connection failed: {error}"),
            Self::TimedOut(error) => write!(formatter, "Ollama request timed out: {error}"),
            Self::Rejected { status, detail } => {
                write!(
                    formatter,
                    "Ollama rejected the request with HTTP {status}: {detail}"
                )
            }
            Self::ModelUnavailable => write!(formatter, "selected Ollama model was not found"),
            Self::InvalidResponse(detail) => write!(formatter, "invalid Ollama response: {detail}"),
            Self::GenerationFailed(detail) => {
                write!(formatter, "Ollama generation failed: {detail}")
            }
        }
    }
}

impl std::error::Error for ProviderError {}

impl ProviderError {
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::InvalidUrl => "Enter a valid Ollama server URL.",
            Self::Unreachable(_) => {
                "Could not connect to Ollama. Check the server address and make sure Ollama is running."
            }
            Self::TimedOut(_) => "Ollama took too long to respond. Check that the server is reachable.",
            Self::Rejected { .. } => "Ollama responded with an error. Check the server and try again.",
            Self::ModelUnavailable => "The selected model is no longer available. Choose another model.",
            Self::InvalidResponse(_) => "Ollama sent an unexpected response. Try updating Ollama or selecting another model.",
            Self::GenerationFailed(_) => "Generation failed. Try again or choose another model.",
        }
    }
}
