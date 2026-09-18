use crate::domain::{
    character::CharacterDraft,
    conversation::{ConversationMessage, MessageRole},
    persona::Persona,
};

pub fn portrait_prompt_messages(draft: &CharacterDraft) -> Vec<ConversationMessage> {
    vec![
        system("Turn NyxAI character data into one concise, visual Stable Diffusion portrait prompt. Focus on visible appearance, clothing, expression, pose, lighting, and aesthetic. Do not mention chat, roleplay, template variables, dialogue, or invisible personality traits. Return only the visual prompt."),
        user(format!(
            "Name: {}\nDescription: {}\nPersonality (only use visually useful clues): {}\nScenario (only if visually useful): {}",
            draft.name, draft.description, draft.personality, draft.scenario
        )),
    ]
}

pub fn scene_prompt_messages(
    character_name: &str,
    character_description: &str,
    scenario: &str,
    persona: Option<&Persona>,
    recent_messages: &[ConversationMessage],
) -> Vec<ConversationMessage> {
    let recent = recent_messages
        .iter()
        .rev()
        .filter(|message| !matches!(message.role, MessageRole::System))
        .take(4)
        .cloned()
        .collect::<Vec<_>>();
    let history = recent
        .iter()
        .rev()
        .map(|message| format!("{:?}: {}", message.role, message.content))
        .collect::<Vec<_>>()
        .join("\n");
    let persona = persona
        .map(|persona| format!("{} — {}", persona.name, persona.description))
        .unwrap_or_else(|| "User (no visual details supplied)".to_owned());

    vec![
        system("Create one concise Stable Diffusion prompt for the current visible roleplay scene. Describe visible people, appearance, action, expression, environment, lighting, mood, and meaningful objects. Do not include dialogue, internal thoughts, invisible lore, instructions, or meta commentary. Return only the image prompt."),
        user(format!(
            "Character: {character_name}\nCharacter appearance: {character_description}\nScenario: {scenario}\nUser persona: {persona}\nRecent scene:\n{history}"
        )),
    ]
}

fn system(content: impl Into<String>) -> ConversationMessage {
    ConversationMessage {
        role: MessageRole::System,
        content: content.into(),
    }
}

fn user(content: impl Into<String>) -> ConversationMessage {
    ConversationMessage {
        role: MessageRole::User,
        content: content.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portrait_context_uses_current_draft_fields() {
        let draft = CharacterDraft {
            name: "Nyx".to_owned(),
            description: "Pale skin, silver hair".to_owned(),
            ..CharacterDraft::default()
        };
        let messages = portrait_prompt_messages(&draft);
        assert!(messages[1].content.contains("silver hair"));
    }

    #[test]
    fn scene_context_limits_history_to_recent_visible_messages() {
        let history = (0..6)
            .map(|index| ConversationMessage {
                role: MessageRole::Assistant,
                content: format!("scene {index}"),
            })
            .collect::<Vec<_>>();
        let messages = scene_prompt_messages("Nyx", "silver hair", "library", None, &history);
        assert!(!messages[1].content.contains("scene 0"));
        assert!(messages[1].content.contains("scene 5"));
    }
}
