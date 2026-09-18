use anyhow::{bail, Result};

use crate::domain::{
    character::Character,
    chat::PersistedMessage,
    conversation::{ConversationMessage, MessageRole},
    lorebook::{lore_context, resolve_lore_entries, LorebookEntry, ResolvedLoreEntry},
    memory::{memory_context, ResolvedMemoryEntry},
    persona::Persona,
    settings::GenerationSettings,
    template::{resolve_template, TemplateContext},
};

const CHARS_PER_ESTIMATED_TOKEN: usize = 4;
const MESSAGE_OVERHEAD_TOKENS: usize = 4;
const MINIMUM_INPUT_BUDGET: usize = 128;
const RECENT_LORE_MESSAGE_WINDOW: usize = 6;

#[derive(Clone, Debug)]
pub struct PromptBuild {
    pub messages: Vec<ConversationMessage>,
    pub lore_used: Vec<ResolvedLoreEntry>,
    pub memories_used: Vec<ResolvedMemoryEntry>,
}

/// Builds the non-visible character context. Prompt policy stays here rather
/// than being scattered across chat routes or provider implementations.
pub fn build_character_system_prompt(character: &Character, persona: Option<&Persona>) -> String {
    let template_context = TemplateContext::new(
        &character.name,
        persona.map(|persona| persona.name.as_str()),
    );
    let mut sections = vec![format!(
        "You are roleplaying as {}. Stay in character, respect the scenario, and maintain continuity with the conversation.",
        character.name
    )];

    if !character.system_prompt.trim().is_empty() {
        sections.push(format!(
            "Character instructions:\n{}",
            resolve_template(character.system_prompt.trim(), template_context)
        ));
    }
    if !character.description.trim().is_empty() {
        sections.push(format!(
            "Character description:\n{}",
            resolve_template(character.description.trim(), template_context)
        ));
    }
    if !character.personality.trim().is_empty() {
        sections.push(format!(
            "Personality:\n{}",
            resolve_template(character.personality.trim(), template_context)
        ));
    }
    if !character.scenario.trim().is_empty() {
        sections.push(format!(
            "Scenario:\n{}",
            resolve_template(character.scenario.trim(), template_context)
        ));
    }
    match persona {
        Some(persona) if !persona.description.trim().is_empty() => sections.push(format!(
            "User persona:\nName: {}\nDescription: {}",
            persona.name.trim(),
            persona.description.trim()
        )),
        Some(persona) => sections.push(format!("User persona:\nName: {}", persona.name.trim())),
        None => sections.push("User persona:\nName: User".to_owned()),
    }
    if !character.example_dialogue.trim().is_empty() {
        sections.push(format!(
            "Example dialogue (style reference, not recent conversation):\n{}",
            resolve_template(character.example_dialogue.trim(), template_context)
        ));
    }
    if !character.tags.is_empty() {
        sections.push(format!("Character tags: {}", character.tags.join(", ")));
    }

    sections.join("\n\n")
}

/// Uses a conservative character-count approximation when no tokenizer is
/// available. The character system context is retained before older messages;
/// recent history is then selected newest-first and restored chronologically.
#[cfg(test)]
pub fn build_chat_request_messages(
    character: &Character,
    persona: Option<&Persona>,
    history: &[PersistedMessage],
    generation: &GenerationSettings,
) -> Result<Vec<ConversationMessage>> {
    Ok(build_chat_request_with_lore(character, persona, history, generation, &[])?.messages)
}

/// Lore is resolved once per generation against only the newest bounded chat
/// window. Static character context is protected first; lore receives at most
/// one third of the input budget and older conversation is dropped before it.
#[allow(dead_code)]
pub fn build_chat_request_with_lore(
    character: &Character,
    persona: Option<&Persona>,
    history: &[PersistedMessage],
    generation: &GenerationSettings,
    lore_entries: &[LorebookEntry],
) -> Result<PromptBuild> {
    build_chat_request_with_context(character, persona, history, generation, lore_entries, &[])
}

/// Builds the bounded character, lore, semantic-memory, and chat context in
/// one place. Both lore and memories are supplied by their resolvers so this
/// prompt layer remains provider-agnostic.
pub fn build_chat_request_with_context(
    character: &Character,
    persona: Option<&Persona>,
    history: &[PersistedMessage],
    generation: &GenerationSettings,
    lore_entries: &[LorebookEntry],
    memory_entries: &[ResolvedMemoryEntry],
) -> Result<PromptBuild> {
    let context_length = generation.context_length as usize;
    let maximum_response = generation.max_response_length as usize;
    if context_length <= maximum_response.saturating_add(MINIMUM_INPUT_BUDGET) {
        bail!(
            "Increase context length or lower response length to leave room for the conversation."
        );
    }
    let input_budget = context_length - maximum_response;
    let base_system_prompt = trim_to_token_budget(
        &build_character_system_prompt(character, persona),
        input_budget,
    );
    let base_system_cost = estimated_message_tokens(&base_system_prompt);
    let lore_budget = input_budget
        .saturating_sub(base_system_cost)
        .min(input_budget / 3)
        .min(1_024);
    let template_context = TemplateContext::new(
        &character.name,
        persona.map(|persona| persona.name.as_str()),
    );
    let lore_used = resolve_lore_entries(
        lore_entries,
        &history
            .iter()
            .rev()
            .take(RECENT_LORE_MESSAGE_WINDOW)
            .cloned()
            .collect::<Vec<_>>(),
        template_context,
        lore_budget,
    );
    let lore_prompt = lore_context(&lore_used);
    let after_lore_cost = base_system_cost + estimated_message_tokens(&lore_prompt);
    let memory_budget = input_budget
        .saturating_sub(after_lore_cost)
        .min(input_budget / 4)
        .min(768);
    let resolved_memory_entries = memory_entries
        .iter()
        .map(|memory| ResolvedMemoryEntry {
            id: memory.id.clone(),
            content: resolve_template(&memory.content, template_context),
        })
        .collect::<Vec<_>>();
    let mut memories_used = Vec::new();
    let mut memory_spent = 0usize;
    for memory in &resolved_memory_entries {
        let cost = estimated_message_tokens(&memory.content) + 3;
        if memory_spent + cost <= memory_budget {
            memory_spent += cost;
            memories_used.push(memory.clone());
        }
    }
    let memory_prompt = memory_context(&memories_used);
    let system_prompt = if lore_prompt.is_empty() && memory_prompt.is_empty() {
        base_system_prompt
    } else {
        [base_system_prompt, lore_prompt, memory_prompt]
            .into_iter()
            .filter(|section| !section.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    let system_cost = estimated_message_tokens(&system_prompt);
    let mut remaining_budget = input_budget.saturating_sub(system_cost);
    let mut selected = Vec::new();

    for message in history.iter().rev() {
        let cost = estimated_message_tokens(&message.content);
        if cost <= remaining_budget {
            selected.push(ConversationMessage {
                role: message.role,
                content: message.content.clone(),
            });
            remaining_budget -= cost;
        }
    }
    selected.reverse();

    let Some(newest_message) = history.last() else {
        bail!("A chat needs a user message before it can be generated.");
    };
    let newest_was_kept = selected.last().is_some_and(|message| {
        message.role == newest_message.role && message.content == newest_message.content
    });
    if !newest_was_kept || newest_message.role != MessageRole::User {
        bail!("The newest message is too long for the configured context length.");
    }

    let mut request = Vec::with_capacity(selected.len() + 1);
    request.push(ConversationMessage {
        role: MessageRole::System,
        content: system_prompt,
    });
    request.extend(selected);
    Ok(PromptBuild {
        messages: request,
        lore_used,
        memories_used,
    })
}

fn estimated_message_tokens(content: &str) -> usize {
    content.chars().count().div_ceil(CHARS_PER_ESTIMATED_TOKEN) + MESSAGE_OVERHEAD_TOKENS
}

fn trim_to_token_budget(content: &str, token_budget: usize) -> String {
    let character_budget =
        token_budget.saturating_sub(MESSAGE_OVERHEAD_TOKENS) * CHARS_PER_ESTIMATED_TOKEN;
    if content.chars().count() <= character_budget {
        return content.to_owned();
    }
    content.chars().take(character_budget).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn character(name: &str) -> Character {
        Character {
            id: "a0000000-0000-4000-8000-000000000001".to_owned(),
            name: name.to_owned(),
            avatar_path: None,
            avatar_url: None,
            description: "A curious stargazer".to_owned(),
            personality: "Gentle and precise".to_owned(),
            scenario: "A midnight observatory".to_owned(),
            first_message: String::new(),
            example_dialogue: "Nyx: The sky is listening.".to_owned(),
            system_prompt: "Never break character.".to_owned(),
            creator_notes: String::new(),
            tags: vec!["companion".to_owned()],
            alternate_greetings: Vec::new(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn message(chat_id: &str, role: MessageRole, content: &str) -> PersistedMessage {
        PersistedMessage {
            id: format!("a0000000-0000-4000-8000-0000000000{chat_id}"),
            chat_id: chat_id.to_owned(),
            role,
            content: content.to_owned(),
            created_at: String::new(),
        }
    }

    fn persona(name: &str) -> Persona {
        Persona {
            id: "a0000000-0000-4000-8000-000000000010".to_owned(),
            name: name.to_owned(),
            avatar_path: None,
            avatar_url: None,
            description: "A quiet traveler".to_owned(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn builds_character_context_without_empty_headings() {
        let prompt = build_character_system_prompt(&character("Nyx"), None);
        assert!(prompt.starts_with("You are roleplaying as Nyx."));
        assert!(prompt.contains("Character instructions:"));
        assert!(prompt.contains("Example dialogue (style reference"));
        assert!(!prompt.contains("First message:"));
    }

    #[test]
    fn keeps_only_the_active_chat_history() {
        let settings = GenerationSettings::default();
        let chat_a = vec![message("1", MessageRole::User, "Message from chat A")];
        let chat_b = vec![message("2", MessageRole::User, "Message from chat B")];
        let built_a = build_chat_request_messages(&character("Nyx"), None, &chat_a, &settings)
            .expect("chat A context");
        let built_b = build_chat_request_messages(&character("Luna"), None, &chat_b, &settings)
            .expect("chat B context");

        assert!(built_a
            .iter()
            .any(|message| message.content.contains("chat A")));
        assert!(!built_a
            .iter()
            .any(|message| message.content.contains("chat B")));
        assert!(built_b
            .iter()
            .any(|message| message.content.contains("chat B")));
        assert!(!built_b
            .iter()
            .any(|message| message.content.contains("chat A")));
    }

    #[test]
    fn drops_old_history_before_the_character_context() {
        let settings = GenerationSettings {
            context_length: 640,
            max_response_length: 256,
            ..GenerationSettings::default()
        };
        let history = vec![
            message("1", MessageRole::User, &"old ".repeat(500)),
            message("1", MessageRole::Assistant, "Most recent reply"),
            message("1", MessageRole::User, "Newest question"),
        ];

        let messages = build_chat_request_messages(&character("Nyx"), None, &history, &settings)
            .expect("trimmed context");
        assert!(messages[0].content.contains("roleplaying as Nyx"));
        assert!(messages
            .iter()
            .any(|message| message.content == "Newest question"));
        assert!(!messages
            .iter()
            .any(|message| message.content.starts_with("old ")));
    }

    #[test]
    fn resolves_character_fields_per_persona_without_mutating_raw_data() {
        let mut nyx = character("Nyx");
        nyx.system_prompt = "{{char}} trusts {{user}}.".to_owned();
        nyx.example_dialogue = "{{char}}: Welcome, {{user}}.".to_owned();
        let arturo = persona("Arturo");
        let aldric = persona("Aldric");

        let arturo_prompt = build_character_system_prompt(&nyx, Some(&arturo));
        let aldric_prompt = build_character_system_prompt(&nyx, Some(&aldric));

        assert!(arturo_prompt.contains("Nyx trusts Arturo."));
        assert!(aldric_prompt.contains("Nyx trusts Aldric."));
        assert_eq!(nyx.system_prompt, "{{char}} trusts {{user}}.");
        assert_eq!(nyx.example_dialogue, "{{char}}: Welcome, {{user}}.");
    }

    #[test]
    fn each_chat_builds_its_own_persona_context() {
        let mut nyx = character("Nyx");
        nyx.description = "{{char}} greets {{user}}.".to_owned();
        let settings = GenerationSettings::default();
        let chat_a = vec![message("a", MessageRole::User, "Arturo's question")];
        let chat_b = vec![message("b", MessageRole::User, "Aldric's question")];
        let arturo = persona("Arturo");
        let aldric = persona("Aldric");

        let request_a = build_chat_request_messages(&nyx, Some(&arturo), &chat_a, &settings)
            .expect("Arturo request");
        let request_b = build_chat_request_messages(&nyx, Some(&aldric), &chat_b, &settings)
            .expect("Aldric request");

        assert!(request_a[0].content.contains("Nyx greets Arturo."));
        assert!(!request_a[0].content.contains("Aldric"));
        assert!(request_b[0].content.contains("Nyx greets Aldric."));
        assert!(!request_b[0].content.contains("Arturo"));
        assert_eq!(nyx.description, "{{char}} greets {{user}}.");
    }

    #[test]
    fn lore_uses_only_the_recent_window_and_preserves_static_context() {
        let lore = LorebookEntry {
            id: "a0000000-0000-4000-8000-000000000101".to_owned(),
            lorebook_id: "a0000000-0000-4000-8000-000000000102".to_owned(),
            name: "Blackwood".to_owned(),
            keys: vec!["blackwood".to_owned()],
            content: "{{char}} knows {{user}} from Blackwood.".to_owned(),
            enabled: true,
            priority: 100,
            case_sensitive: false,
            match_mode: crate::domain::lorebook::LoreMatchMode::AnyKey,
            always_active: false,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let mut history = vec![message(
            "lore",
            MessageRole::User,
            "Tell me about Blackwood.",
        )];
        for index in 0..5 {
            history.push(message(
                "recent",
                MessageRole::Assistant,
                &format!("Reply {index}"),
            ));
        }
        history.push(message("recent", MessageRole::User, "Newest question"));

        let without_recent_match = build_chat_request_with_lore(
            &character("Nyx"),
            Some(&persona("Arturo")),
            &history,
            &GenerationSettings::default(),
            std::slice::from_ref(&lore),
        )
        .expect("prompt should build");
        assert!(without_recent_match.lore_used.is_empty());

        history.last_mut().expect("newest message").content = "Blackwood is nearby.".to_owned();
        let with_recent_match = build_chat_request_with_lore(
            &character("Nyx"),
            Some(&persona("Arturo")),
            &history,
            &GenerationSettings::default(),
            std::slice::from_ref(&lore),
        )
        .expect("prompt should build");
        assert_eq!(with_recent_match.lore_used.len(), 1);
        assert!(with_recent_match.messages[0]
            .content
            .contains("Nyx knows Arturo from Blackwood."));
        assert_eq!(lore.content, "{{char}} knows {{user}} from Blackwood.");
    }

    #[test]
    fn memory_context_is_separate_bounded_and_resolves_templates() {
        let settings = GenerationSettings {
            context_length: 1_024,
            max_response_length: 256,
            ..GenerationSettings::default()
        };
        let memories = vec![
            ResolvedMemoryEntry {
                id: "memory-a".to_owned(),
                content: "{{char}} promised {{user}} a place in Blackwood.".to_owned(),
            },
            ResolvedMemoryEntry {
                id: "memory-b".to_owned(),
                content: "x ".repeat(2_000),
            },
        ];
        let history = vec![message(
            "memory",
            MessageRole::User,
            "What did you promise?",
        )];
        let build = build_chat_request_with_context(
            &character("Nyx"),
            Some(&persona("Arturo")),
            &history,
            &settings,
            &[],
            &memories,
        )
        .expect("memory prompt should build");
        assert_eq!(build.memories_used.len(), 1);
        assert!(build.messages[0]
            .content
            .contains("Nyx promised Arturo a place in Blackwood."));
        assert!(build.messages[0]
            .content
            .contains("[Relevant Semantic Memory]"));
    }
}
