use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::domain::{
    chat::PersistedMessage,
    template::{resolve_template, TemplateContext},
};

const MAX_NAME_LENGTH: usize = 120;
const MAX_DESCRIPTION_LENGTH: usize = 4_000;
const MAX_CONTENT_LENGTH: usize = 32_000;
const MAX_KEYS: usize = 40;
const MAX_KEY_LENGTH: usize = 160;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lorebook {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoreMatchMode {
    #[default]
    AnyKey,
    AllKeys,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LorebookEntry {
    pub id: String,
    pub lorebook_id: String,
    pub name: String,
    pub keys: Vec<String>,
    pub content: String,
    pub enabled: bool,
    pub priority: i64,
    pub case_sensitive: bool,
    pub match_mode: LoreMatchMode,
    pub always_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LorebookDraft {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LorebookEntryDraft {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub keys: Vec<String>,
    #[serde(default)]
    pub content: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_priority")]
    pub priority: i64,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub match_mode: LoreMatchMode,
    #[serde(default)]
    pub always_active: bool,
}

fn default_enabled() -> bool {
    true
}
fn default_priority() -> i64 {
    50
}

impl Default for LorebookDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            enabled: true,
        }
    }
}

impl Default for LorebookEntryDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            keys: Vec::new(),
            content: String::new(),
            enabled: true,
            priority: default_priority(),
            case_sensitive: false,
            match_mode: LoreMatchMode::AnyKey,
            always_active: false,
        }
    }
}

impl LorebookDraft {
    pub fn normalize_and_validate(mut self) -> Result<Self> {
        self.name = self.name.trim().to_owned();
        self.description = self.description.trim().to_owned();
        if self.name.is_empty() {
            bail!("Give the lorebook a name.");
        }
        if self.name.chars().count() > MAX_NAME_LENGTH {
            bail!("Lorebook names can be at most {MAX_NAME_LENGTH} characters.");
        }
        if self.description.chars().count() > MAX_DESCRIPTION_LENGTH {
            bail!("Lorebook descriptions can be at most {MAX_DESCRIPTION_LENGTH} characters.");
        }
        Ok(self)
    }
}

impl LorebookEntryDraft {
    pub fn normalize_and_validate(mut self) -> Result<Self> {
        self.name = self.name.trim().to_owned();
        self.content = self.content.trim().to_owned();
        if self.name.is_empty() {
            bail!("Give the lore entry a name.");
        }
        if self.name.chars().count() > MAX_NAME_LENGTH {
            bail!("Lore entry names can be at most {MAX_NAME_LENGTH} characters.");
        }
        if self.content.is_empty() {
            bail!("Add the world information for this entry.");
        }
        if self.content.chars().count() > MAX_CONTENT_LENGTH {
            bail!("Lore content can be at most {MAX_CONTENT_LENGTH} characters.");
        }
        let mut keys = Vec::new();
        for key in self.keys {
            let key = key.trim();
            if key.is_empty() {
                continue;
            }
            if key.chars().count() > MAX_KEY_LENGTH {
                bail!("Lore keys can be at most {MAX_KEY_LENGTH} characters.");
            }
            if !keys
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(key))
            {
                keys.push(key.to_owned());
            }
        }
        if keys.len() > MAX_KEYS {
            bail!("Use at most {MAX_KEYS} lore keys.");
        }
        if !self.always_active && keys.is_empty() {
            bail!("Add a trigger key or mark this entry Always Active.");
        }
        self.keys = keys;
        Ok(self)
    }
}

impl From<LorebookEntry> for LorebookEntryDraft {
    fn from(entry: LorebookEntry) -> Self {
        Self {
            name: entry.name,
            keys: entry.keys,
            content: entry.content,
            enabled: entry.enabled,
            priority: entry.priority,
            case_sensitive: entry.case_sensitive,
            match_mode: entry.match_mode,
            always_active: entry.always_active,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResolvedLoreEntry {
    pub id: String,
    pub name: String,
    pub content: String,
    pub priority: i64,
}

/// Deterministically selects lore from a bounded recent history. It does not
/// examine previously injected lore, execute expressions, or mutate content.
pub fn resolve_lore_entries(
    entries: &[LorebookEntry],
    recent_messages: &[PersistedMessage],
    context: TemplateContext<'_>,
    token_budget: usize,
) -> Vec<ResolvedLoreEntry> {
    let haystack = recent_messages
        .iter()
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let mut matches = entries
        .iter()
        .filter(|entry| entry.enabled && !entry.content.trim().is_empty())
        .filter(|entry| entry.always_active || entry_matches(entry, &haystack))
        .map(|entry| ResolvedLoreEntry {
            id: entry.id.clone(),
            name: entry.name.clone(),
            content: resolve_template(&entry.content, context),
            priority: entry.priority,
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.id.cmp(&right.id))
    });
    let mut selected = Vec::new();
    let mut spent = 0usize;
    for entry in matches {
        let cost = estimated_tokens(&entry.name) + estimated_tokens(&entry.content) + 4;
        if spent + cost <= token_budget {
            spent += cost;
            selected.push(entry);
        }
    }
    selected
}

pub fn lore_context(entries: &[ResolvedLoreEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let rendered = entries
        .iter()
        .map(|entry| format!("{}:\n{}", entry.name, entry.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    format!("[Relevant World Information]\n{rendered}")
}

fn estimated_tokens(value: &str) -> usize {
    value.chars().count().div_ceil(4) + 4
}

fn entry_matches(entry: &LorebookEntry, haystack: &str) -> bool {
    let matched = entry
        .keys
        .iter()
        .filter(|key| phrase_matches(haystack, key, entry.case_sensitive))
        .count();
    match entry.match_mode {
        LoreMatchMode::AnyKey => matched > 0,
        LoreMatchMode::AllKeys => matched == entry.keys.len() && !entry.keys.is_empty(),
    }
}

/// Avoids false positives such as `vale` inside `available` while still
/// allowing natural multi-word phrases and punctuation boundaries.
fn phrase_matches(source: &str, key: &str, case_sensitive: bool) -> bool {
    let key = key.trim();
    if key.is_empty() {
        return false;
    }
    let source = if case_sensitive {
        source.to_owned()
    } else {
        source.to_lowercase()
    };
    let needle = if case_sensitive {
        key.to_owned()
    } else {
        key.to_lowercase()
    };
    let mut start = 0;
    while let Some(found) = source[start..].find(&needle) {
        let index = start + found;
        let before = source[..index].chars().next_back();
        let after = source[index + needle.len()..].chars().next();
        if before.is_none_or(|character| !is_word(character))
            && after.is_none_or(|character| !is_word(character))
        {
            return true;
        }
        start = index + needle.len();
    }
    false
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::conversation::MessageRole;

    fn entry(name: &str, keys: &[&str], priority: i64) -> LorebookEntry {
        LorebookEntry {
            id: name.to_owned(),
            lorebook_id: "book".to_owned(),
            name: name.to_owned(),
            keys: keys.iter().map(|key| (*key).to_owned()).collect(),
            content: "{{char}} knows {{user}}.".to_owned(),
            enabled: true,
            priority,
            case_sensitive: false,
            match_mode: LoreMatchMode::AnyKey,
            always_active: false,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
    fn message(content: &str) -> PersistedMessage {
        PersistedMessage {
            id: "id".to_owned(),
            chat_id: "chat".to_owned(),
            role: MessageRole::User,
            content: content.to_owned(),
            created_at: String::new(),
        }
    }

    #[test]
    fn matches_boundaries_and_templates_without_mutating_lore() {
        let source = entry("House Vale", &["vale"], 50);
        let matches = resolve_lore_entries(
            std::slice::from_ref(&source),
            &[message("Tell me about Vale.")],
            TemplateContext::new("Nyx", Some("Arturo")),
            200,
        );
        assert_eq!(matches.len(), 1);
        assert!(matches[0].content.contains("Nyx knows Arturo"));
        assert_eq!(source.content, "{{char}} knows {{user}}.");
        assert!(resolve_lore_entries(
            &[source],
            &[message("available")],
            TemplateContext::new("Nyx", None),
            200
        )
        .is_empty());
    }
    #[test]
    fn supports_case_and_always_active_priority() {
        let mut strict = entry("Strict", &["Vale"], 10);
        strict.case_sensitive = true;
        let mut always = entry("World rules", &[], 100);
        always.always_active = true;
        let matches = resolve_lore_entries(
            &[strict.clone(), always],
            &[message("vale")],
            TemplateContext::new("Nyx", None),
            200,
        );
        assert_eq!(
            matches
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["World rules"]
        );
        strict.case_sensitive = false;
        assert_eq!(
            resolve_lore_entries(
                &[strict],
                &[message("vale")],
                TemplateContext::new("Nyx", None),
                200
            )
            .len(),
            1
        );
    }
    #[test]
    fn all_keys_and_budget_are_deterministic() {
        let mut all = entry("Both", &["blackwood", "vale"], 50);
        all.match_mode = LoreMatchMode::AllKeys;
        let low = entry("Low", &["vale"], 10);
        let result = resolve_lore_entries(
            &[low, all],
            &[message("Blackwood belongs to Vale")],
            TemplateContext::new("Nyx", None),
            20,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Both");
    }
}
