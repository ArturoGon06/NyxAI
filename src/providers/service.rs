use std::pin::Pin;

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use serde::Serialize;

use crate::{
    domain::{
        conversation::ConversationMessage,
        settings::{GenerationSettings, ProviderKind},
    },
    providers::{ollama::OllamaProvider, openai_compatible::OpenAICompatibleProvider},
};

pub type ProviderStream = Pin<Box<dyn Stream<Item = Result<StreamingChunk, ProviderError>> + Send>>;

/// Capability used by chat, Character Creator, Memory Extraction, and image
/// prompt construction. Callers never learn a backend-specific HTTP route.
#[async_trait]
pub trait TextGenerationProvider: Send + Sync {
    async fn stream_chat(
        &self,
        base_url: &str,
        model: &str,
        messages: Vec<ConversationMessage>,
        generation: GenerationSettings,
    ) -> Result<ProviderStream, ProviderError>;
}

/// Model discovery is separate because a local server may accept a manual
/// model name even if it does not expose a discovery endpoint.
#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn list_models(&self, base_url: &str) -> Result<Vec<ProviderModel>, ProviderError>;
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(
        &self,
        base_url: &str,
        model: &str,
        input: &str,
    ) -> Result<Vec<f32>, ProviderError>;
}

#[async_trait]
pub trait ModelLifecycleProvider: Send + Sync {
    async fn unload_model(&self, base_url: &str, model: &str) -> Result<(), ProviderError>;
    async fn preload_model(&self, base_url: &str, model: &str) -> Result<(), ProviderError>;
}

/// The one place that knows which concrete adapters NyxAI ships. New local
/// backends register here; routes, prompt construction, and memory code stay
/// provider-neutral.
#[derive(Clone)]
struct BackendRegistry {
    ollama: OllamaProvider,
    openai_compatible: OpenAICompatibleProvider,
}

impl BackendRegistry {
    fn resolve(&self, provider: &ProviderKind) -> BackendAdapter<'_> {
        match provider {
            ProviderKind::Ollama => BackendAdapter::Ollama(&self.ollama),
            ProviderKind::OpenaiCompatible => {
                BackendAdapter::OpenAICompatible(&self.openai_compatible)
            }
        }
    }
}

enum BackendAdapter<'a> {
    Ollama(&'a OllamaProvider),
    OpenAICompatible(&'a OpenAICompatibleProvider),
}

impl BackendAdapter<'_> {
    fn capabilities(&self) -> ProviderCapabilities {
        match self {
            Self::Ollama(_) => ProviderCapabilities {
                connection_testing: true,
                model_listing: true,
                // Ollama exposes installed models directly, so retain its
                // existing availability validation instead of accepting an
                // arbitrary model name.
                manual_model_entry: false,
                chat_generation: true,
                streaming_generation: true,
                generation_cancellation: true,
                basic_generation_settings: true,
                embeddings: true,
                model_lifecycle: true,
            },
            Self::OpenAICompatible(_) => ProviderCapabilities {
                connection_testing: true,
                // GET /models is part of the compatible protocol, but some
                // local servers omit it. The UI retains manual entry.
                model_listing: true,
                manual_model_entry: true,
                chat_generation: true,
                streaming_generation: true,
                // Dropping the client stream stops NyxAI's request, but this
                // protocol has no portable server-side interrupt endpoint.
                generation_cancellation: false,
                basic_generation_settings: true,
                embeddings: true,
                model_lifecycle: false,
            },
        }
    }
}

#[async_trait]
impl TextGenerationProvider for BackendAdapter<'_> {
    async fn stream_chat(
        &self,
        base_url: &str,
        model: &str,
        messages: Vec<ConversationMessage>,
        generation: GenerationSettings,
    ) -> Result<ProviderStream, ProviderError> {
        match self {
            Self::Ollama(adapter) => {
                adapter
                    .stream_chat(base_url, model, messages, generation)
                    .await
            }
            Self::OpenAICompatible(adapter) => {
                adapter
                    .stream_chat(base_url, model, messages, generation)
                    .await
            }
        }
    }
}

#[async_trait]
impl ModelProvider for BackendAdapter<'_> {
    async fn list_models(&self, base_url: &str) -> Result<Vec<ProviderModel>, ProviderError> {
        match self {
            Self::Ollama(adapter) => adapter.list_models(base_url).await,
            Self::OpenAICompatible(adapter) => adapter.list_models(base_url).await,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for BackendAdapter<'_> {
    async fn embed(
        &self,
        base_url: &str,
        model: &str,
        input: &str,
    ) -> Result<Vec<f32>, ProviderError> {
        match self {
            Self::Ollama(adapter) => adapter.embed(base_url, model, input).await,
            Self::OpenAICompatible(adapter) => adapter.embed(base_url, model, input).await,
        }
    }
}

#[async_trait]
impl ModelLifecycleProvider for BackendAdapter<'_> {
    async fn unload_model(&self, base_url: &str, model: &str) -> Result<(), ProviderError> {
        match self {
            Self::Ollama(adapter) => adapter.unload_model(base_url, model).await,
            Self::OpenAICompatible(_) => {
                Err(ProviderError::UnsupportedCapability("model unloading"))
            }
        }
    }

    async fn preload_model(&self, base_url: &str, model: &str) -> Result<(), ProviderError> {
        match self {
            Self::Ollama(adapter) => adapter.preload_model(base_url, model).await,
            Self::OpenAICompatible(_) => {
                Err(ProviderError::UnsupportedCapability("model preloading"))
            }
        }
    }
}

#[derive(Clone)]
pub struct ProviderService {
    registry: BackendRegistry,
}

impl ProviderService {
    /// The optional token only becomes an HTTP authorization header inside the
    /// adapter. It is never sent to the browser or included in error values.
    pub fn new(openai_compatible_api_key: Option<String>) -> anyhow::Result<Self> {
        Ok(Self {
            registry: BackendRegistry {
                ollama: OllamaProvider::new()?,
                openai_compatible: OpenAICompatibleProvider::new(openai_compatible_api_key)?,
            },
        })
    }

    pub fn capabilities(&self, provider: &ProviderKind) -> ProviderCapabilities {
        self.registry.resolve(provider).capabilities()
    }

    pub async fn embed(
        &self,
        provider: &ProviderKind,
        base_url: &str,
        model: &str,
        input: &str,
    ) -> Result<Vec<f32>, ProviderError> {
        let adapter = self.registry.resolve(provider);
        if !adapter.capabilities().embeddings {
            return Err(ProviderError::UnsupportedCapability("embeddings"));
        }
        adapter.embed(base_url, model, input).await
    }

    pub async fn test_connection(
        &self,
        provider: &ProviderKind,
        base_url: &str,
    ) -> Result<ConnectionReport, ProviderError> {
        let models = match self.list_models(provider, base_url).await {
            Ok(models) => models,
            // A compatible server that answers GET /models with 404 is still
            // demonstrably reachable. It simply needs manual model entry.
            Err(ProviderError::UnsupportedCapability("model discovery"))
                if self.capabilities(provider).manual_model_entry =>
            {
                Vec::new()
            }
            Err(error) => return Err(error),
        };
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
        let adapter = self.registry.resolve(provider);
        if !adapter.capabilities().model_listing {
            return Err(ProviderError::UnsupportedCapability("model discovery"));
        }
        adapter.list_models(base_url).await
    }

    pub async fn stream_chat(
        &self,
        provider: &ProviderKind,
        base_url: &str,
        model: &str,
        messages: Vec<ConversationMessage>,
        generation: GenerationSettings,
    ) -> Result<ProviderStream, ProviderError> {
        self.registry
            .resolve(provider)
            .stream_chat(base_url, model, messages, generation)
            .await
    }

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
        let adapter = self.registry.resolve(provider);
        if !adapter.capabilities().model_lifecycle {
            return Err(ProviderError::UnsupportedCapability("model unloading"));
        }
        adapter.unload_model(base_url, model).await
    }

    pub async fn preload_model(
        &self,
        provider: &ProviderKind,
        base_url: &str,
        model: &str,
    ) -> Result<(), ProviderError> {
        let adapter = self.registry.resolve(provider);
        if !adapter.capabilities().model_lifecycle {
            return Err(ProviderError::UnsupportedCapability("model preloading"));
        }
        adapter.preload_model(base_url, model).await
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderCapabilities {
    pub connection_testing: bool,
    pub model_listing: bool,
    pub manual_model_entry: bool,
    pub chat_generation: bool,
    pub streaming_generation: bool,
    pub generation_cancellation: bool,
    pub basic_generation_settings: bool,
    pub embeddings: bool,
    pub model_lifecycle: bool,
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
    AuthenticationFailed,
    Rejected { status: u16 },
    ModelUnavailable,
    UnsupportedCapability(&'static str),
    InvalidResponse(String),
    GenerationFailed(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrl => write!(formatter, "invalid inference-server URL"),
            Self::Unreachable(error) => {
                write!(formatter, "inference-server connection failed: {error}")
            }
            Self::TimedOut(error) => {
                write!(formatter, "inference-server request timed out: {error}")
            }
            Self::AuthenticationFailed => {
                write!(formatter, "inference-server authentication failed")
            }
            Self::Rejected { status } => write!(
                formatter,
                "inference server rejected the request with HTTP {status}"
            ),
            Self::ModelUnavailable => write!(formatter, "selected model was not found"),
            Self::UnsupportedCapability(capability) => {
                write!(formatter, "backend does not support {capability}")
            }
            Self::InvalidResponse(detail) => {
                write!(formatter, "invalid inference-server response: {detail}")
            }
            Self::GenerationFailed(detail) => {
                write!(formatter, "inference generation failed: {detail}")
            }
        }
    }
}
impl std::error::Error for ProviderError {}

impl ProviderError {
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::InvalidUrl => "Enter a valid local inference server URL.",
            Self::Unreachable(_) => "Could not connect to the inference backend. Check the server address and make sure it is running.",
            Self::TimedOut(_) => "The inference backend took too long to respond. Check that it is reachable.",
            Self::AuthenticationFailed => "The inference backend rejected its optional API key. Check the server configuration.",
            Self::Rejected { .. } => "The inference backend responded with an error. Check the server and try again.",
            Self::ModelUnavailable => "The selected model is no longer available. Choose another model.",
            Self::UnsupportedCapability(_) => "This inference backend does not support that capability.",
            Self::InvalidResponse(_) => "The inference backend sent an unexpected response. Try another local server or model.",
            Self::GenerationFailed(_) => "Generation failed. Try again or choose another model.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[test]
    fn registry_selects_ollama_and_openai_compatible_capabilities() {
        let service = ProviderService::new(None).expect("service should initialize");
        assert!(service.capabilities(&ProviderKind::Ollama).model_lifecycle);
        let compatible = service.capabilities(&ProviderKind::OpenaiCompatible);
        assert!(compatible.manual_model_entry);
        assert!(!compatible.model_lifecycle);
    }

    #[test]
    fn unsupported_lifecycle_is_a_clear_capability_error() {
        let error = ProviderError::UnsupportedCapability("model unloading");
        assert_eq!(
            error.user_message(),
            "This inference backend does not support that capability."
        );
        assert!(!error.to_string().contains("token"));
    }

    #[tokio::test]
    async fn compatible_lifecycle_calls_fail_without_contacting_a_server() {
        let service = ProviderService::new(None).expect("service should initialize");
        let error = service
            .unload_model(
                &ProviderKind::OpenaiCompatible,
                "http://127.0.0.1:1/v1",
                "local-model",
            )
            .await
            .expect_err("compatible lifecycle is unsupported");
        assert!(matches!(
            error,
            ProviderError::UnsupportedCapability("model unloading")
        ));
    }

    #[tokio::test]
    async fn compatible_server_without_model_discovery_is_still_connectable() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener");
        let address = listener.local_addr().expect("listener address");
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("test connection");
            let mut request = [0_u8; 1024];
            let _ = socket.read(&mut request).await;
            socket
                .write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await
                .expect("test response");
        });

        let service = ProviderService::new(None).expect("service should initialize");
        let report = service
            .test_connection(
                &ProviderKind::OpenaiCompatible,
                &format!("http://{address}/v1"),
            )
            .await
            .expect("404 discovery indicates a reachable compatible server");
        assert!(report.connected);
        assert_eq!(report.model_count, 0);
    }
}
