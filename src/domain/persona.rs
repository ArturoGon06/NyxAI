use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

const MAX_PERSONA_NAME_LENGTH: usize = 120;
const MAX_PERSONA_DESCRIPTION_LENGTH: usize = 32_000;

/// A user's in-story identity. Personas are intentionally small: prompt
/// construction needs a stable name and optional description, not a second
/// character-card format.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Persona {
    pub id: String,
    pub name: String,
    pub avatar_path: Option<String>,
    pub avatar_url: Option<String>,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PersonaDraft {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub avatar_path: Option<String>,
    #[serde(default)]
    pub description: String,
}

impl PersonaDraft {
    pub fn normalize_and_validate(mut self) -> Result<Self> {
        self.name = self.name.trim().to_owned();
        if self.name.is_empty() {
            bail!("Give the persona a name.");
        }
        if self.name.chars().count() > MAX_PERSONA_NAME_LENGTH {
            bail!("Persona names can be at most {MAX_PERSONA_NAME_LENGTH} characters.");
        }
        if self.description.chars().count() > MAX_PERSONA_DESCRIPTION_LENGTH {
            bail!(
                "Persona descriptions can be at most {MAX_PERSONA_DESCRIPTION_LENGTH} characters."
            );
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_a_valid_persona_name() {
        let persona = PersonaDraft {
            name: " Arturo ".to_owned(),
            description: "A quiet traveler".to_owned(),
            ..PersonaDraft::default()
        }
        .normalize_and_validate()
        .expect("persona should validate");

        assert_eq!(persona.name, "Arturo");
    }

    #[test]
    fn rejects_a_blank_persona_name() {
        assert!(PersonaDraft::default().normalize_and_validate().is_err());
    }
}
