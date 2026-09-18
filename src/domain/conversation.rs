use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// Provider-facing conversation message assembled from persisted chat data.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

impl MessageRole {
    pub fn as_database_value(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }

    pub fn from_database_value(value: &str) -> Result<Self> {
        match value {
            "system" => Ok(Self::System),
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            _ => bail!("stored message role is invalid"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConversationMessage {
    pub role: MessageRole,
    pub content: String,
}

pub fn validate_message_content(content: &str) -> Result<()> {
    if content.trim().is_empty() || content.chars().count() > 64_000 {
        bail!("Messages must contain text and be shorter than 64,000 characters.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_blank_or_oversized_message_content() {
        assert!(validate_message_content("  ").is_err());
        assert!(validate_message_content(&"x".repeat(64_001)).is_err());
        assert!(validate_message_content("Hello").is_ok());
    }
}
