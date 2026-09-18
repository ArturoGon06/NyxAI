use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::domain::{
    chat::PersistedMessage,
    conversation::{ConversationMessage, MessageRole},
    template::{resolve_template, TemplateContext},
};

const MAX_MEMORY_CONTENT: usize = 2_000;
const MAX_CANDIDATES: usize = 12;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    #[default]
    Chat,
    Character,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub character_id: String,
    pub chat_id: Option<String>,
    pub scope: MemoryScope,
    pub content: String,
    pub importance: i64,
    pub manually_created: bool,
    pub source_start_message_id: Option<String>,
    pub source_end_message_id: Option<String>,
    pub embedding_model: Option<String>,
    pub embedding_dimension: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MemoryDraft {
    #[serde(default)]
    pub chat_id: Option<String>,
    #[serde(default)]
    pub scope: MemoryScope,
    #[serde(default)]
    pub content: String,
    #[serde(default = "default_importance")]
    pub importance: i64,
    #[serde(default)]
    pub source_start_message_id: Option<String>,
    #[serde(default)]
    pub source_end_message_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct MemoryCandidate {
    pub content: String,
    pub importance: i64,
}

#[derive(Clone, Debug, Deserialize)]
struct MemoryCandidateEnvelope {
    #[serde(default)]
    memories: Vec<MemoryCandidate>,
}

#[derive(Clone, Debug)]
pub struct ResolvedMemoryEntry {
    pub id: String,
    pub content: String,
}

pub fn default_importance() -> i64 {
    60
}

impl MemoryDraft {
    pub fn normalize_and_validate(mut self) -> Result<Self> {
        self.content = normalize_memory_content(&self.content);
        if self.content.is_empty() {
            bail!("Add a memory before saving.");
        }
        if self.content.chars().count() > MAX_MEMORY_CONTENT {
            bail!("Memories can be at most {MAX_MEMORY_CONTENT} characters.");
        }
        if !(1..=100).contains(&self.importance) {
            bail!("Memory importance must be between 1 and 100.");
        }
        if matches!(self.scope, MemoryScope::Character) {
            self.chat_id = None;
        }
        Ok(self)
    }
}

pub fn normalize_memory_content(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn normalized_memory_key(value: &str) -> String {
    normalize_memory_content(value).to_lowercase()
}

pub fn parse_memory_candidates(output: &str) -> Result<Vec<MemoryCandidate>> {
    let json =
        extract_json_object(output).ok_or_else(|| anyhow::anyhow!("memory output was not JSON"))?;
    let envelope: MemoryCandidateEnvelope = serde_json::from_str(json)
        .map_err(|error| anyhow::anyhow!("memory output could not be read: {error}"))?;
    let mut candidates = Vec::new();
    for candidate in envelope.memories.into_iter().take(MAX_CANDIDATES) {
        let content = normalize_memory_content(&candidate.content);
        if content.is_empty() || content.chars().count() > MAX_MEMORY_CONTENT {
            continue;
        }
        if !(1..=100).contains(&candidate.importance) {
            continue;
        }
        if !candidates.iter().any(|existing: &MemoryCandidate| {
            normalized_memory_key(&existing.content) == normalized_memory_key(&content)
        }) {
            candidates.push(MemoryCandidate {
                content,
                importance: candidate.importance,
            });
        }
    }
    Ok(candidates)
}

pub fn extraction_messages(messages: &[PersistedMessage]) -> Vec<ConversationMessage> {
    let transcript = messages
        .iter()
        .map(|message| format!("{}: {}", role_label(message.role), message.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    vec![
        ConversationMessage {
            role: MessageRole::System,
            content: "Extract only concise, durable roleplay memories from this conversation. Include significant events, revealed facts, relationship changes, promises, lasting preferences, discoveries, plans, important objects, or world changes. Do not extract greetings, filler, routine movements, formatting, trivial dialogue, or invented facts. The memories must stand alone and accurately reflect the transcript. Return JSON only: {\"memories\":[{\"content\":\"...\",\"importance\":1-100}]}. Return an empty array when nothing deserves long-term memory.".to_owned(),
        },
        ConversationMessage { role: MessageRole::User, content: transcript },
    ]
}

pub fn extraction_repair_messages(invalid_output: &str) -> Vec<ConversationMessage> {
    vec![
        ConversationMessage {
            role: MessageRole::System,
            content: "Repair the following output into only valid JSON matching {\"memories\":[{\"content\":\"...\",\"importance\":1-100}]}. Preserve only concise, durable facts. Do not add commentary.".to_owned(),
        },
        ConversationMessage { role: MessageRole::User, content: invalid_output.to_owned() },
    ]
}

pub fn build_memory_query(
    character_name: &str,
    scenario: &str,
    history: &[PersistedMessage],
) -> String {
    let recent = history
        .iter()
        .rev()
        .take(4)
        .rev()
        .map(|message| format!("{}: {}", role_label(message.role), message.content))
        .collect::<Vec<_>>()
        .join("\n");
    format!("Character: {character_name}\nScenario: {scenario}\nRecent conversation:\n{recent}")
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> Option<f32> {
    if left.is_empty()
        || left.len() != right.len()
        || !left.iter().chain(right).all(|v| v.is_finite())
    {
        return None;
    }
    let mut dot = 0.0_f64;
    let mut left_norm = 0.0_f64;
    let mut right_norm = 0.0_f64;
    for (a, b) in left.iter().zip(right) {
        dot += f64::from(*a) * f64::from(*b);
        left_norm += f64::from(*a) * f64::from(*a);
        right_norm += f64::from(*b) * f64::from(*b);
    }
    if left_norm <= f64::EPSILON || right_norm <= f64::EPSILON {
        return None;
    }
    Some((dot / (left_norm.sqrt() * right_norm.sqrt())) as f32)
}

pub fn select_memory_context(
    candidates: Vec<(MemoryEntry, f32)>,
    context: TemplateContext<'_>,
    threshold: f32,
    top_k: usize,
    token_budget: usize,
) -> Vec<ResolvedMemoryEntry> {
    let mut ranked = candidates
        .into_iter()
        .filter(|(_, similarity)| similarity.is_finite() && *similarity >= threshold)
        .map(|(entry, similarity)| {
            let score = similarity + (entry.importance as f32 / 100.0) * 0.08;
            (entry, similarity, score)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .2
            .total_cmp(&left.2)
            .then_with(|| right.1.total_cmp(&left.1))
            .then_with(|| left.0.id.cmp(&right.0.id))
    });
    let mut selected = Vec::new();
    let mut spent = 0usize;
    for (entry, _similarity, _) in ranked.into_iter().take(top_k) {
        let content = resolve_template(&entry.content, context);
        let cost = estimated_tokens(&content) + 3;
        if spent + cost <= token_budget {
            spent += cost;
            selected.push(ResolvedMemoryEntry {
                id: entry.id,
                content,
            });
        }
    }
    selected
}

pub fn memory_context(memories: &[ResolvedMemoryEntry]) -> String {
    if memories.is_empty() {
        return String::new();
    }
    let list = memories
        .iter()
        .map(|memory| format!("- {}", memory.content))
        .collect::<Vec<_>>()
        .join("\n");
    format!("[Relevant Semantic Memory]\n{list}")
}

fn estimated_tokens(value: &str) -> usize {
    value.chars().count().div_ceil(4) + 4
}
fn role_label(role: MessageRole) -> &'static str {
    match role {
        MessageRole::User => "User",
        MessageRole::Assistant => "Character",
        MessageRole::System => "System",
    }
}

fn extract_json_object(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed);
    }
    let fence = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"));
    if let Some(fenced) = fence.and_then(|value| value.strip_suffix("```")) {
        let candidate = fenced.trim();
        if candidate.starts_with('{') && candidate.ends_with('}') {
            return Some(candidate);
        }
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    (end > start).then(|| &trimmed[start..=end])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(content: &str, importance: i64) -> MemoryEntry {
        MemoryEntry {
            id: content.to_owned(),
            character_id: "character".to_owned(),
            chat_id: Some("chat".to_owned()),
            scope: MemoryScope::Chat,
            content: content.to_owned(),
            importance,
            manually_created: false,
            source_start_message_id: None,
            source_end_message_id: None,
            embedding_model: Some("embed".to_owned()),
            embedding_dimension: Some(2),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn parses_fenced_candidates_and_rejects_malformed_output() {
        let candidates = parse_memory_candidates(
            "```json\n{\"memories\":[{\"content\":\"Lexi fears storms.\",\"importance\":75}]}\n```",
        )
        .expect("fenced JSON should parse");
        assert_eq!(candidates.len(), 1);
        assert!(parse_memory_candidates("not JSON").is_err());
    }

    #[test]
    fn cosine_threshold_and_importance_stay_conservative() {
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]), Some(1.0));
        assert!(cosine_similarity(&[1.0], &[1.0, 0.0]).is_none());
        let memories = select_memory_context(
            vec![(entry("Relevant", 50), 0.8), (entry("Unrelated", 100), 0.2)],
            TemplateContext::new("Nyx", Some("Arturo")),
            0.5,
            4,
            100,
        );
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].content, "Relevant");
    }
}
