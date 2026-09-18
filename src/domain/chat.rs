use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::domain::conversation::{validate_message_content, MessageRole};

const DEFAULT_CHAT_TITLE: &str = "New Conversation";
const MAX_CHAT_TITLE_LENGTH: usize = 120;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chat {
    pub id: String,
    pub character_id: String,
    pub persona_id: Option<String>,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedMessage {
    pub id: String,
    pub chat_id: String,
    pub role: MessageRole,
    pub content: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CreateChatRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub greeting_id: Option<String>,
    /// Missing means use the global default (or the one available persona).
    /// A JSON null is an explicit no-persona selection for this chat.
    #[serde(default)]
    pub persona_id: Option<Option<String>>,
}

impl CreateChatRequest {
    pub fn validated_title(self) -> Result<String> {
        validate_title(self.title.as_deref().unwrap_or(DEFAULT_CHAT_TITLE))
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RenameChatRequest {
    pub title: String,
}

impl RenameChatRequest {
    pub fn validated_title(self) -> Result<String> {
        validate_title(&self.title)
    }
}

pub fn validate_title(title: &str) -> Result<String> {
    let title = title.trim();
    if title.is_empty() {
        bail!("Give the chat a title.");
    }
    if title.chars().count() > MAX_CHAT_TITLE_LENGTH {
        bail!("Chat titles can be at most {MAX_CHAT_TITLE_LENGTH} characters.");
    }
    Ok(title.to_owned())
}

pub fn validate_persisted_message(role: MessageRole, content: &str) -> Result<()> {
    validate_message_content(content)?;
    if matches!(role, MessageRole::System) {
        bail!("System context is generated from the character and is not a visible chat message.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_new_chats_to_a_clear_title() {
        assert_eq!(
            CreateChatRequest::default()
                .validated_title()
                .expect("title"),
            "New Conversation"
        );
    }

    #[test]
    fn rejects_blank_chat_titles() {
        assert!(validate_title("   ").is_err());
    }
}
