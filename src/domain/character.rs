use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

const MAX_NAME_LENGTH: usize = 120;
const MAX_TEXT_LENGTH: usize = 32_000;
const MAX_TAGS: usize = 20;
const MAX_TAG_LENGTH: usize = 40;
const MAX_ALTERNATE_GREETINGS: usize = 20;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CharacterGreeting {
    pub id: String,
    pub content: String,
    pub position: i64,
}

/// NyxAI's normalized character shape. External card formats are intentionally
/// kept outside this model so future importers can normalize into these fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub avatar_path: Option<String>,
    pub avatar_url: Option<String>,
    pub description: String,
    pub personality: String,
    pub scenario: String,
    pub first_message: String,
    pub example_dialogue: String,
    pub system_prompt: String,
    pub creator_notes: String,
    pub tags: Vec<String>,
    pub alternate_greetings: Vec<CharacterGreeting>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CharacterSummary {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub description: String,
    pub tags: Vec<String>,
    pub updated_at: String,
}

impl From<&Character> for CharacterSummary {
    fn from(character: &Character) -> Self {
        Self {
            id: character.id.clone(),
            name: character.name.clone(),
            avatar_url: character.avatar_url.clone(),
            description: character.description.clone(),
            tags: character.tags.clone(),
            updated_at: character.updated_at.clone(),
        }
    }
}

/// Fields accepted when creating or replacing a character. The independent
/// prompt fields are stored separately for later prompt construction.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CharacterDraft {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub avatar_path: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub scenario: String,
    #[serde(default)]
    pub first_message: String,
    #[serde(default)]
    pub example_dialogue: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub creator_notes: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub alternate_greetings: Vec<String>,
}

impl CharacterDraft {
    pub fn normalize_and_validate(mut self) -> Result<Self> {
        self.name = self.name.trim().to_owned();
        if self.name.is_empty() {
            bail!("Give the character a name.");
        }
        if self.name.chars().count() > MAX_NAME_LENGTH {
            bail!("Character names can be at most {MAX_NAME_LENGTH} characters.");
        }

        for value in [
            &self.description,
            &self.personality,
            &self.scenario,
            &self.first_message,
            &self.example_dialogue,
            &self.system_prompt,
            &self.creator_notes,
        ] {
            if value.chars().count() > MAX_TEXT_LENGTH {
                bail!("Character text fields can be at most {MAX_TEXT_LENGTH} characters.");
            }
        }

        let mut normalized_tags = Vec::new();
        for tag in self.tags {
            let tag = tag.trim();
            if tag.is_empty() {
                continue;
            }
            if tag.chars().count() > MAX_TAG_LENGTH {
                bail!("Tags can be at most {MAX_TAG_LENGTH} characters.");
            }
            if !normalized_tags
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(tag))
            {
                normalized_tags.push(tag.to_owned());
            }
        }

        if normalized_tags.len() > MAX_TAGS {
            bail!("Use at most {MAX_TAGS} tags.");
        }
        self.tags = normalized_tags;

        let mut normalized_greetings = Vec::new();
        for greeting in self.alternate_greetings {
            let greeting = greeting.trim();
            if greeting.is_empty() {
                continue;
            }
            if greeting.chars().count() > MAX_TEXT_LENGTH {
                bail!("Alternate greetings can be at most {MAX_TEXT_LENGTH} characters.");
            }
            normalized_greetings.push(greeting.to_owned());
        }
        if normalized_greetings.len() > MAX_ALTERNATE_GREETINGS {
            bail!("Use at most {MAX_ALTERNATE_GREETINGS} alternate greetings.");
        }
        self.alternate_greetings = normalized_greetings;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_blank_name() {
        assert!(CharacterDraft::default().normalize_and_validate().is_err());
    }

    #[test]
    fn normalizes_and_deduplicates_tags() {
        let draft = CharacterDraft {
            name: " Nyx ".to_owned(),
            tags: vec![
                " companion ".to_owned(),
                "COMPANION".to_owned(),
                "".to_owned(),
            ],
            ..CharacterDraft::default()
        }
        .normalize_and_validate()
        .expect("draft should validate");

        assert_eq!(draft.name, "Nyx");
        assert_eq!(draft.tags, vec!["companion"]);
    }

    #[test]
    fn preserves_valid_alternate_greetings() {
        let draft = CharacterDraft {
            name: "Nyx".to_owned(),
            alternate_greetings: vec![" Welcome. ".to_owned(), "Another greeting.".to_owned()],
            ..CharacterDraft::default()
        }
        .normalize_and_validate()
        .expect("draft should validate");

        assert_eq!(
            draft.alternate_greetings,
            vec!["Welcome.", "Another greeting."]
        );
    }
}
