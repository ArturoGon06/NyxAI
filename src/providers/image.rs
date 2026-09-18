use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::domain::image::ImageGenerationRequest;

#[derive(Clone)]
pub struct ImageProviderService {
    a1111: A1111ImageProvider,
}

#[derive(Clone)]
pub struct A1111ImageProvider {
    client: Client,
}

#[derive(Debug, Clone)]
pub struct GeneratedImage {
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageBackendReport {
    pub connected: bool,
    pub model_count: Option<usize>,
}

#[derive(Debug)]
pub enum ImageGenerationError {
    InvalidUrl,
    Unreachable(reqwest::Error),
    TimedOut(reqwest::Error),
    Rejected { status: u16, detail: String },
    InvalidResponse(String),
    InvalidImage,
}

impl ImageGenerationError {
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::InvalidUrl => "The A1111 server address is invalid.",
            Self::Unreachable(_) => "Could not reach the image backend. Check its address and make sure its API is enabled.",
            Self::TimedOut(_) => "Image generation timed out. Try a smaller image or check the image backend.",
            Self::Rejected { .. } => "The image backend rejected this generation request.",
            Self::InvalidResponse(_) | Self::InvalidImage => "The image backend returned an invalid image.",
        }
    }
}

impl std::fmt::Display for ImageGenerationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUrl => write!(formatter, "invalid A1111 URL"),
            Self::Unreachable(error) | Self::TimedOut(error) => {
                write!(formatter, "image backend request failed: {error}")
            }
            Self::Rejected { status, detail } => write!(
                formatter,
                "image backend rejected request ({status}): {detail}"
            ),
            Self::InvalidResponse(detail) => {
                write!(formatter, "invalid image backend response: {detail}")
            }
            Self::InvalidImage => {
                write!(formatter, "image backend returned unsupported image data")
            }
        }
    }
}

impl std::error::Error for ImageGenerationError {}

impl ImageProviderService {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            a1111: A1111ImageProvider::new()?,
        })
    }

    pub async fn test_connection(
        &self,
        base_url: &str,
    ) -> Result<ImageBackendReport, ImageGenerationError> {
        self.a1111.test_connection(base_url).await
    }

    pub async fn list_models(&self, base_url: &str) -> Result<Vec<String>, ImageGenerationError> {
        self.a1111.list_models(base_url).await
    }

    pub async fn generate(
        &self,
        base_url: &str,
        request: ImageGenerationRequest,
    ) -> Result<GeneratedImage, ImageGenerationError> {
        self.a1111.generate(base_url, request).await
    }

    pub async fn interrupt(&self, base_url: &str) -> Result<(), ImageGenerationError> {
        self.a1111.empty_post(base_url, "sdapi/v1/interrupt").await
    }

    /// A1111 exposes this optional endpoint. A failure is only a cleanup warning.
    pub async fn release_memory(&self, base_url: &str) -> Result<(), ImageGenerationError> {
        self.a1111
            .empty_post(base_url, "sdapi/v1/unload-checkpoint")
            .await
    }
}

impl A1111ImageProvider {
    fn new() -> anyhow::Result<Self> {
        Ok(Self {
            client: Client::builder()
                .connect_timeout(Duration::from_secs(8))
                .timeout(Duration::from_secs(180))
                .build()?,
        })
    }

    async fn test_connection(
        &self,
        base_url: &str,
    ) -> Result<ImageBackendReport, ImageGenerationError> {
        let models = self.list_models(base_url).await;
        match models {
            Ok(models) => Ok(ImageBackendReport {
                connected: true,
                model_count: Some(models.len()),
            }),
            // Some compatible backends omit checkpoint discovery but do expose options.
            Err(_) => {
                let endpoint = endpoint(base_url, "sdapi/v1/options")?;
                let response = self
                    .client
                    .get(endpoint)
                    .send()
                    .await
                    .map_err(network_error)?;
                ensure_success(response).await?;
                Ok(ImageBackendReport {
                    connected: true,
                    model_count: None,
                })
            }
        }
    }

    async fn list_models(&self, base_url: &str) -> Result<Vec<String>, ImageGenerationError> {
        let endpoint = endpoint(base_url, "sdapi/v1/sd-models")?;
        let response = ensure_success(
            self.client
                .get(endpoint)
                .send()
                .await
                .map_err(network_error)?,
        )
        .await?;
        let models: Vec<A1111Model> = response
            .json()
            .await
            .map_err(|error| ImageGenerationError::InvalidResponse(error.to_string()))?;
        let mut names = models
            .into_iter()
            .filter_map(|model| model.title.or(model.model_name))
            .filter(|name| !name.trim().is_empty())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        Ok(names)
    }

    async fn generate(
        &self,
        base_url: &str,
        request: ImageGenerationRequest,
    ) -> Result<GeneratedImage, ImageGenerationError> {
        request
            .validate()
            .map_err(|error| ImageGenerationError::InvalidResponse(error.to_string()))?;
        let endpoint = endpoint(base_url, "sdapi/v1/txt2img")?;
        let payload = A1111TextToImageRequest::from(request);
        let response = ensure_success(
            self.client
                .post(endpoint)
                .json(&payload)
                .send()
                .await
                .map_err(network_error)?,
        )
        .await?;
        let body: A1111TextToImageResponse = response
            .json()
            .await
            .map_err(|error| ImageGenerationError::InvalidResponse(error.to_string()))?;
        let encoded = body.images.into_iter().next().ok_or_else(|| {
            ImageGenerationError::InvalidResponse("no images were returned".to_owned())
        })?;
        let encoded = encoded
            .rsplit_once(',')
            .map(|(_, value)| value)
            .unwrap_or(&encoded);
        let bytes = STANDARD
            .decode(encoded.trim())
            .map_err(|error| ImageGenerationError::InvalidResponse(error.to_string()))?;
        if !is_supported_image(&bytes) {
            return Err(ImageGenerationError::InvalidImage);
        }
        Ok(GeneratedImage { bytes })
    }

    async fn empty_post(&self, base_url: &str, path: &str) -> Result<(), ImageGenerationError> {
        let endpoint = endpoint(base_url, path)?;
        ensure_success(
            self.client
                .post(endpoint)
                .json(&serde_json::json!({}))
                .send()
                .await
                .map_err(network_error)?,
        )
        .await?;
        Ok(())
    }
}

fn endpoint(base_url: &str, path: &str) -> Result<Url, ImageGenerationError> {
    let mut base = Url::parse(base_url.trim()).map_err(|_| ImageGenerationError::InvalidUrl)?;
    if !matches!(base.scheme(), "http" | "https") || base.host_str().is_none() {
        return Err(ImageGenerationError::InvalidUrl);
    }
    base.set_query(None);
    base.set_fragment(None);
    if !base.path().ends_with('/') {
        base.set_path(&format!("{}/", base.path()));
    }
    base.join(path)
        .map_err(|_| ImageGenerationError::InvalidUrl)
}

fn network_error(error: reqwest::Error) -> ImageGenerationError {
    if error.is_timeout() {
        ImageGenerationError::TimedOut(error)
    } else {
        ImageGenerationError::Unreachable(error)
    }
}

async fn ensure_success(
    response: reqwest::Response,
) -> Result<reqwest::Response, ImageGenerationError> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status().as_u16();
    let detail = response.text().await.unwrap_or_default();
    Err(ImageGenerationError::Rejected { status, detail })
}

fn is_supported_image(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes.starts_with(&[0xff, 0xd8, 0xff])
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP")
}

#[derive(Deserialize)]
struct A1111Model {
    title: Option<String>,
    model_name: Option<String>,
}
#[derive(Deserialize)]
struct A1111TextToImageResponse {
    #[serde(default)]
    images: Vec<String>,
}

#[derive(Serialize)]
struct A1111TextToImageRequest {
    prompt: String,
    negative_prompt: String,
    width: u32,
    height: u32,
    steps: u32,
    cfg_scale: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    sampler_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    override_settings: Option<OverrideSettings>,
}
#[derive(Serialize)]
struct OverrideSettings {
    sd_model_checkpoint: String,
}
impl From<ImageGenerationRequest> for A1111TextToImageRequest {
    fn from(request: ImageGenerationRequest) -> Self {
        Self {
            prompt: request.prompt,
            negative_prompt: request.negative_prompt,
            width: request.width,
            height: request.height,
            steps: request.steps,
            cfg_scale: request.cfg_scale,
            sampler_name: request.sampler_name,
            override_settings: request.model.map(|sd_model_checkpoint| OverrideSettings {
                sd_model_checkpoint,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_a1111_base64_image_data() {
        let bytes = STANDARD.decode("iVBORw0KGgo=").unwrap();
        assert!(is_supported_image(&bytes));
    }

    #[test]
    fn parses_the_first_a1111_image_from_a_response_shape() {
        let response: A1111TextToImageResponse =
            serde_json::from_str(r#"{"images":["data:image/png;base64,iVBORw0KGgo=","ignored"]}"#)
                .expect("response shape should deserialize");
        let encoded = response.images.first().unwrap().rsplit_once(',').unwrap().1;
        assert!(is_supported_image(&STANDARD.decode(encoded).unwrap()));
    }
    #[test]
    fn rejects_unsafe_backend_urls() {
        assert!(endpoint("file:///tmp", "sdapi/v1/txt2img").is_err());
    }
    #[test]
    fn maps_only_the_supported_txt2img_request_fields() {
        let request = A1111TextToImageRequest::from(ImageGenerationRequest {
            prompt: "portrait".to_owned(),
            negative_prompt: String::new(),
            width: 512,
            height: 512,
            steps: 20,
            cfg_scale: 7.0,
            sampler_name: None,
            model: Some("local.safetensors".to_owned()),
        });
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(
            value["override_settings"]["sd_model_checkpoint"],
            "local.safetensors"
        );
    }
}
