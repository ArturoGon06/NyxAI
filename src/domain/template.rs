/// The values supported by NyxAI's intentionally small, plain-text template
/// vocabulary. Unknown placeholders are left untouched for card portability.
#[derive(Clone, Copy, Debug)]
pub struct TemplateContext<'a> {
    pub character_name: &'a str,
    pub persona_name: Option<&'a str>,
}

impl<'a> TemplateContext<'a> {
    pub fn new(character_name: &'a str, persona_name: Option<&'a str>) -> Self {
        Self {
            character_name,
            persona_name,
        }
    }

    pub fn user_name(self) -> &'a str {
        self.persona_name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("User")
    }
}

/// Resolves only documented identifiers. This is not a scripting language and
/// does not evaluate expressions, recurse into substituted values, or remove
/// unrecognized placeholders.
pub fn resolve_template(source: &str, context: TemplateContext<'_>) -> String {
    source
        .replace("{{char}}", context.character_name)
        .replace("{{Char}}", context.character_name)
        .replace("{{user}}", context.user_name())
        .replace("{{User}}", context.user_name())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_character_and_persona_without_mutating_the_source() {
        let raw = "{{char}} greets {{user}}.";
        let resolved = resolve_template(raw, TemplateContext::new("Nyx", Some("Arturo")));

        assert_eq!(raw, "{{char}} greets {{user}}.");
        assert_eq!(resolved, "Nyx greets Arturo.");
    }

    #[test]
    fn supports_common_capitalization_and_preserves_unknown_variables() {
        let resolved = resolve_template(
            "{{Char}} meets {{User}} near {{world}}.",
            TemplateContext::new("Nyx", Some("Aldric")),
        );

        assert_eq!(resolved, "Nyx meets Aldric near {{world}}.");
    }

    #[test]
    fn uses_a_non_empty_fallback_when_no_persona_is_selected() {
        assert_eq!(
            resolve_template("Hello, {{user}}.", TemplateContext::new("Nyx", None)),
            "Hello, User."
        );
    }
}
