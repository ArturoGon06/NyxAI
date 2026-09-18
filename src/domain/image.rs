use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

pub const MAX_IMAGE_PROMPT_LENGTH: usize = 4_000;
pub const MAX_IMAGE_DIMENSION: u32 = 2_048;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatImage {
    pub id: String,
    pub chat_id: String,
    pub image_url: String,
    pub prompt: String,
    pub model: Option<String>,
    pub width: u32,
    pub height: u32,
    pub created_at: String,
}

#[derive(Clone, Debug)]
pub struct ImageGenerationRequest {
    pub prompt: String,
    pub negative_prompt: String,
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub cfg_scale: f32,
    pub sampler_name: Option<String>,
    pub model: Option<String>,
}

impl ImageGenerationRequest {
    pub fn validate(&self) -> Result<()> {
        if self.prompt.trim().is_empty() || self.prompt.chars().count() > MAX_IMAGE_PROMPT_LENGTH {
            bail!("The image prompt must contain text and be shorter than {MAX_IMAGE_PROMPT_LENGTH} characters.");
        }
        validate_dimensions(self.width, self.height)?;
        if !(1..=100).contains(&self.steps) {
            bail!("Image steps must be between 1 and 100.");
        }
        if !(1.0..=30.0).contains(&self.cfg_scale) {
            bail!("Image CFG scale must be between 1 and 30.");
        }
        Ok(())
    }
}

pub fn validate_dimensions(width: u32, height: u32) -> Result<()> {
    if !(128..=MAX_IMAGE_DIMENSION).contains(&width)
        || !(128..=MAX_IMAGE_DIMENSION).contains(&height)
        || width % 8 != 0
        || height % 8 != 0
    {
        bail!("Image dimensions must be 128–{MAX_IMAGE_DIMENSION} pixels and divisible by 8.");
    }
    Ok(())
}

pub fn normalize_visual_prompt(value: &str) -> Result<String> {
    let value = value.trim().trim_matches('`').trim();
    if value.is_empty() {
        bail!("The image prompt model returned no visual description.");
    }
    let value = value
        .strip_prefix("Prompt:")
        .or_else(|| value.strip_prefix("prompt:"))
        .unwrap_or(value)
        .trim();
    if value.chars().count() > MAX_IMAGE_PROMPT_LENGTH {
        bail!("The generated image prompt was too long.");
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_a_compact_image_request() {
        let request = ImageGenerationRequest {
            prompt: "moonlit library portrait".to_owned(),
            negative_prompt: String::new(),
            width: 512,
            height: 512,
            steps: 28,
            cfg_scale: 7.0,
            sampler_name: None,
            model: None,
        };
        assert!(request.validate().is_ok());
        assert!(validate_dimensions(513, 512).is_err());
    }

    #[test]
    fn trims_prompt_model_wrapping_without_accepting_empty_output() {
        assert_eq!(
            normalize_visual_prompt("`Prompt: warm portrait`").unwrap(),
            "warm portrait"
        );
        assert!(normalize_visual_prompt("  ").is_err());
    }
}
