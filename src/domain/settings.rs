use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct AppSettings {
    pub appearance: AppearanceSettings,
    pub provider: ProviderSettings,
    pub generation: GenerationSettings,
    pub persona: PersonaSettings,
    pub image: ImageSettings,
    pub memory: MemorySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppearanceSettings {
    pub app_background: String,
    pub secondary_background: String,
    pub user_message_background: String,
    pub user_message_text: String,
    pub character_message_background: String,
    pub character_message_text: String,
    pub accent_color: String,
    pub muted_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    #[default]
    Ollama,
    OpenaiCompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ProviderSettings {
    pub active_provider: ProviderKind,
    /// None means use the OLLAMA_BASE_URL environment default.
    pub ollama_base_url: Option<String>,
    /// None means use the OPENAI_COMPATIBLE_BASE_URL environment default.
    /// Credentials stay environment-only and never reach browser settings.
    pub openai_compatible_base_url: Option<String>,
    pub selected_model: Option<String>,
    /// Optional local model dedicated to producing structured character drafts.
    /// When absent, NyxAI falls back to the selected chat model.
    pub character_creator_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GenerationSettings {
    pub temperature: f32,
    pub context_length: u32,
    pub max_response_length: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct PersonaSettings {
    pub default_persona_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ImageSettings {
    pub enabled: bool,
    /// None means use the A1111_BASE_URL environment default.
    pub a1111_base_url: Option<String>,
    pub selected_model: Option<String>,
    pub image_prompt_model: Option<String>,
    pub single_gpu_memory_mode: bool,
    pub restore_text_model_after_generation: bool,
    pub avatar_width: u32,
    pub avatar_height: u32,
    pub chat_width: u32,
    pub chat_height: u32,
    pub steps: u32,
    pub cfg_scale: f32,
    pub sampler_name: Option<String>,
    pub negative_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MemorySettings {
    pub enabled: bool,
    pub automatic_extraction: bool,
    pub embedding_model: Option<String>,
    /// May remain Ollama while text generation uses another local backend.
    pub embedding_provider: ProviderKind,
    pub extraction_model: Option<String>,
    pub extraction_interval: u32,
    pub retrieval_count: u32,
    pub similarity_threshold: f32,
    pub context_budget: u32,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            app_background: "#08080B".to_owned(),
            secondary_background: "#101014".to_owned(),
            user_message_background: "#4A2C63".to_owned(),
            user_message_text: "#F2EEF5".to_owned(),
            character_message_background: "#21172D".to_owned(),
            character_message_text: "#F2EEF5".to_owned(),
            accent_color: "#C9A227".to_owned(),
            muted_text: "#9A94A3".to_owned(),
        }
    }
}

impl Default for ProviderSettings {
    fn default() -> Self {
        Self {
            active_provider: ProviderKind::Ollama,
            ollama_base_url: None,
            openai_compatible_base_url: None,
            selected_model: None,
            character_creator_model: None,
        }
    }
}

impl Default for GenerationSettings {
    fn default() -> Self {
        Self {
            temperature: 0.8,
            context_length: 4096,
            max_response_length: 512,
        }
    }
}

impl Default for ImageSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            a1111_base_url: None,
            selected_model: None,
            image_prompt_model: None,
            single_gpu_memory_mode: false,
            restore_text_model_after_generation: false,
            avatar_width: 512,
            avatar_height: 512,
            chat_width: 768,
            chat_height: 512,
            steps: 28,
            cfg_scale: 7.0,
            sampler_name: None,
            negative_prompt: String::new(),
        }
    }
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            automatic_extraction: true,
            embedding_model: None,
            embedding_provider: ProviderKind::Ollama,
            extraction_model: None,
            extraction_interval: 8,
            retrieval_count: 4,
            similarity_threshold: 0.62,
            context_budget: 512,
        }
    }
}

impl AppSettings {
    pub fn normalize_and_validate(&mut self) -> Result<()> {
        self.appearance.normalize()?;
        self.validate()
    }

    pub fn validate(&self) -> Result<()> {
        let colors = [
            &self.appearance.app_background,
            &self.appearance.secondary_background,
            &self.appearance.user_message_background,
            &self.appearance.user_message_text,
            &self.appearance.character_message_background,
            &self.appearance.character_message_text,
            &self.appearance.accent_color,
            &self.appearance.muted_text,
        ];

        if colors
            .iter()
            .any(|color| normalize_hex_color(color).is_none())
        {
            bail!("Colors must use the #RRGGBB format.");
        }

        for url in [
            &self.provider.ollama_base_url,
            &self.provider.openai_compatible_base_url,
        ]
        .into_iter()
        .flatten()
        {
            validate_provider_url(url)?;
        }

        if let Some(model) = &self.provider.selected_model {
            if model.trim().is_empty() || model.chars().count() > 256 {
                bail!("The selected model name is invalid.");
            }
        }

        if let Some(model) = &self.provider.character_creator_model {
            if model.trim().is_empty() || model.chars().count() > 256 {
                bail!("The Character Creator model name is invalid.");
            }
        }

        if let Some(persona_id) = &self.persona.default_persona_id {
            if uuid::Uuid::parse_str(persona_id).is_err() {
                bail!("The default persona is invalid.");
            }
        }

        if !self.generation.temperature.is_finite()
            || !(0.0..=2.0).contains(&self.generation.temperature)
        {
            bail!("Temperature must be between 0 and 2.");
        }

        if !(512..=131_072).contains(&self.generation.context_length) {
            bail!("Context length must be between 512 and 131072 tokens.");
        }

        if !(1..=32_768).contains(&self.generation.max_response_length) {
            bail!("Maximum response length must be between 1 and 32768 tokens.");
        }

        self.image.validate()?;
        self.memory.validate()?;

        Ok(())
    }
}

impl MemorySettings {
    pub fn validate(&self) -> Result<()> {
        for model in [&self.embedding_model, &self.extraction_model] {
            if model
                .as_ref()
                .is_some_and(|value| value.trim().is_empty() || value.chars().count() > 256)
            {
                bail!("A memory model setting is invalid.");
            }
        }
        if !(4..=32).contains(&self.extraction_interval)
            || !(1..=8).contains(&self.retrieval_count)
            || !self.similarity_threshold.is_finite()
            || !(0.0..=1.0).contains(&self.similarity_threshold)
            || !(64..=2_048).contains(&self.context_budget)
        {
            bail!("Memory settings are invalid.");
        }
        Ok(())
    }
}

impl ImageSettings {
    pub fn validate(&self) -> Result<()> {
        if let Some(url) = &self.a1111_base_url {
            let parsed = Url::parse(url.trim())
                .map_err(|_| anyhow::anyhow!("The A1111 server URL is invalid."))?;
            if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
                bail!("The A1111 server URL is invalid.");
            }
        }
        for model in [
            &self.selected_model,
            &self.image_prompt_model,
            &self.sampler_name,
        ] {
            if model
                .as_ref()
                .is_some_and(|value| value.trim().is_empty() || value.chars().count() > 256)
            {
                bail!("An image model setting is invalid.");
            }
        }
        crate::domain::image::validate_dimensions(self.avatar_width, self.avatar_height)?;
        crate::domain::image::validate_dimensions(self.chat_width, self.chat_height)?;
        if !(1..=100).contains(&self.steps)
            || !self.cfg_scale.is_finite()
            || !(1.0..=30.0).contains(&self.cfg_scale)
        {
            bail!("Image generation settings are invalid.");
        }
        if self.negative_prompt.chars().count() > 4_000 {
            bail!("The negative prompt is too long.");
        }
        Ok(())
    }
}

impl AppearanceSettings {
    pub fn normalize(&mut self) -> Result<()> {
        for color in [
            &mut self.app_background,
            &mut self.secondary_background,
            &mut self.user_message_background,
            &mut self.user_message_text,
            &mut self.character_message_background,
            &mut self.character_message_text,
            &mut self.accent_color,
            &mut self.muted_text,
        ] {
            *color = normalize_hex_color(color)
                .ok_or_else(|| anyhow::anyhow!("Colors must use the #RRGGBB format."))?;
        }
        Ok(())
    }

    /// Saved legacy data should never make the UI unusable. Invalid stored
    /// colors fall back one field at a time; new writes remain strictly valid.
    pub fn normalize_or_default(&mut self) {
        let defaults = Self::default();
        for (color, fallback) in [
            (&mut self.app_background, defaults.app_background),
            (
                &mut self.secondary_background,
                defaults.secondary_background,
            ),
            (
                &mut self.user_message_background,
                defaults.user_message_background,
            ),
            (&mut self.user_message_text, defaults.user_message_text),
            (
                &mut self.character_message_background,
                defaults.character_message_background,
            ),
            (
                &mut self.character_message_text,
                defaults.character_message_text,
            ),
            (&mut self.accent_color, defaults.accent_color),
            (&mut self.muted_text, defaults.muted_text),
        ] {
            *color = normalize_hex_color(color).unwrap_or(fallback);
        }
    }

    /// Deserialize persisted appearance data defensively. Appearance values
    /// are user-customizable, so an old or manually edited settings row must
    /// not keep the application from loading because one color is malformed.
    pub fn from_persisted_value(value: Option<&serde_json::Value>) -> Self {
        let mut appearance = Self::default();
        let Some(fields) = value.and_then(serde_json::Value::as_object) else {
            return appearance;
        };

        for (field, color) in [
            ("app_background", &mut appearance.app_background),
            ("secondary_background", &mut appearance.secondary_background),
            (
                "user_message_background",
                &mut appearance.user_message_background,
            ),
            ("user_message_text", &mut appearance.user_message_text),
            (
                "character_message_background",
                &mut appearance.character_message_background,
            ),
            (
                "character_message_text",
                &mut appearance.character_message_text,
            ),
            ("accent_color", &mut appearance.accent_color),
            ("muted_text", &mut appearance.muted_text),
        ] {
            if let Some(normalized) = fields
                .get(field)
                .and_then(serde_json::Value::as_str)
                .and_then(normalize_hex_color)
            {
                *color = normalized;
            }
        }

        appearance
    }
}

pub fn normalize_hex_color(value: &str) -> Option<String> {
    let value = value.trim();
    if value.len() == 7
        && value.starts_with('#')
        && value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
    {
        return Some(value.to_ascii_uppercase());
    }
    None
}

fn validate_provider_url(value: &str) -> Result<()> {
    let parsed = Url::parse(value.trim())
        .map_err(|_| anyhow::anyhow!("Enter a valid local inference server URL."))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        bail!("Enter a valid local inference server URL using http:// or https://.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_is_valid() {
        assert!(AppSettings::default().validate().is_ok());
    }

    #[test]
    fn rejects_invalid_color_values() {
        let mut settings = AppSettings::default();
        settings.appearance.accent_color = "purple".to_owned();
        assert!(settings.validate().is_err());
    }

    #[test]
    fn normalizes_valid_hex_values_and_rejects_invalid_ones() {
        assert_eq!(normalize_hex_color("#c9a227"), Some("#C9A227".to_owned()));
        assert_eq!(normalize_hex_color("C9A227"), None);
        assert_eq!(normalize_hex_color("#ABC"), None);
    }

    #[test]
    fn rejects_an_invalid_ollama_url() {
        let mut settings = AppSettings::default();
        settings.provider.ollama_base_url = Some("ollama.example".to_owned());
        assert!(settings.validate().is_err());
    }

    #[test]
    fn older_settings_default_to_ollama_for_text_and_embeddings() {
        let settings: AppSettings = serde_json::from_value(serde_json::json!({
            "provider": { "ollama_base_url": "http://127.0.0.1:11434" },
            "memory": { "enabled": true, "embedding_model": "nomic-embed-text" }
        }))
        .expect("older settings should deserialize");
        assert_eq!(settings.provider.active_provider, ProviderKind::Ollama);
        assert_eq!(settings.memory.embedding_provider, ProviderKind::Ollama);
    }

    #[test]
    fn accepts_a_valid_default_persona_id() {
        let mut settings = AppSettings::default();
        settings.persona.default_persona_id =
            Some("a0000000-0000-4000-8000-000000000001".to_owned());
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn accepts_a_dedicated_character_creator_model() {
        let mut settings = AppSettings::default();
        settings.provider.character_creator_model = Some("qwen2.5:7b".to_owned());
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn older_settings_json_uses_new_provider_defaults() {
        let mut settings: AppSettings = serde_json::from_str(
            r##"{"appearance":{"app_background":"#08080B","user_message_background":"#4A2C63","user_message_text":"#F2EEF5","character_message_background":"#21172D","character_message_text":"#F2EEF5","accent_color":"#C9A227"}}"##,
        )
        .expect("older saved settings should deserialize");
        settings.appearance.normalize_or_default();

        assert_eq!(settings.provider, ProviderSettings::default());
        assert_eq!(settings.generation, GenerationSettings::default());
        assert_eq!(settings.persona, PersonaSettings::default());
        assert_eq!(settings.image, ImageSettings::default());
        assert_eq!(settings.memory, MemorySettings::default());
        assert_eq!(settings.appearance.secondary_background, "#101014");
        assert_eq!(settings.appearance.muted_text, "#9A94A3");
    }

    #[test]
    fn invalid_stored_colors_fall_back_without_discarding_valid_ones() {
        let mut appearance = AppearanceSettings {
            app_background: "invalid".to_owned(),
            accent_color: "#ab12cd".to_owned(),
            ..AppearanceSettings::default()
        };
        appearance.normalize_or_default();

        assert_eq!(appearance.app_background, "#08080B");
        assert_eq!(appearance.accent_color, "#AB12CD");
    }

    #[test]
    fn persisted_appearance_salvages_valid_colors_and_defaults_bad_fields() {
        let value = serde_json::json!({
            "app_background": "#12111a",
            "secondary_background": 42,
            "user_message_text": null,
            "accent_color": "#b78d24",
        });

        let appearance = AppearanceSettings::from_persisted_value(Some(&value));

        assert_eq!(appearance.app_background, "#12111A");
        assert_eq!(appearance.accent_color, "#B78D24");
        assert_eq!(appearance.secondary_background, "#101014");
        assert_eq!(appearance.user_message_text, "#F2EEF5");
    }

    #[test]
    fn validates_image_backend_settings_without_weakening_defaults() {
        let mut settings = AppSettings::default();
        settings.image.enabled = true;
        settings.image.a1111_base_url = Some("http://127.0.0.1:7860".to_owned());
        settings.image.avatar_width = 513;
        assert!(settings.validate().is_err());

        settings.image.avatar_width = 512;
        settings.image.chat_width = 768;
        assert!(settings.validate().is_ok());
    }
}
