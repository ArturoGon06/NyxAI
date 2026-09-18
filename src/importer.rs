use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{Map, Value};

use crate::domain::character::CharacterDraft;

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const MAX_CARD_BYTES: usize = 4 * 1024 * 1024;
const MAX_METADATA_BYTES: usize = 1024 * 1024;
const MAX_NAME_LENGTH: usize = 120;
const MAX_TEXT_LENGTH: usize = 32_000;
const MAX_TAGS: usize = 20;
const MAX_TAG_LENGTH: usize = 40;
const MAX_ALTERNATE_GREETINGS: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterCardFormat {
    Json,
    Png,
}

pub struct ImportedCharacter {
    pub draft: CharacterDraft,
    pub avatar_bytes: Option<Vec<u8>>,
    pub format: CharacterCardFormat,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub enum CharacterCardImportError {
    TooLarge,
    UnsupportedFormat,
    InvalidJson,
    NoSupportedData,
    MissingPngMetadata,
}

impl CharacterCardImportError {
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::TooLarge => "Character cards must be smaller than 4 MB.",
            Self::UnsupportedFormat => {
                "Unsupported character card format. Choose a JSON card or a PNG character card."
            }
            Self::InvalidJson => "Character card JSON is invalid.",
            Self::NoSupportedData => "This file does not contain recognizable character card data.",
            Self::MissingPngMetadata => {
                "This PNG does not contain supported character card metadata."
            }
        }
    }
}

impl std::fmt::Display for CharacterCardImportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.user_message())
    }
}

impl std::error::Error for CharacterCardImportError {}

/// Parses only external formats, then returns NyxAI's own draft. No format
/// terminology leaves this module or reaches the database/UI directly.
pub fn parse_character_card(bytes: &[u8]) -> Result<ImportedCharacter, CharacterCardImportError> {
    if bytes.len() > MAX_CARD_BYTES {
        return Err(CharacterCardImportError::TooLarge);
    }
    if bytes.starts_with(PNG_SIGNATURE) {
        let metadata = extract_png_card_metadata(bytes)?;
        let (draft, warnings) = parse_json_draft(&metadata)?;
        return Ok(ImportedCharacter {
            draft,
            avatar_bytes: Some(bytes.to_vec()),
            format: CharacterCardFormat::Png,
            warnings,
        });
    }
    if bytes
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace())
        == Some(b'{')
    {
        let (draft, warnings) = parse_json_draft(bytes)?;
        return Ok(ImportedCharacter {
            draft,
            avatar_bytes: None,
            format: CharacterCardFormat::Json,
            warnings,
        });
    }
    Err(CharacterCardImportError::UnsupportedFormat)
}

fn parse_json_draft(
    bytes: &[u8],
) -> Result<(CharacterDraft, Vec<String>), CharacterCardImportError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| CharacterCardImportError::InvalidJson)?;
    let root = value
        .as_object()
        .ok_or(CharacterCardImportError::NoSupportedData)?;
    let data = root
        .get("data")
        .and_then(Value::as_object)
        .or_else(|| root.get("character").and_then(Value::as_object))
        .unwrap_or(root);

    let recognized_fields = [
        "name",
        "char_name",
        "description",
        "personality",
        "char_persona",
        "scenario",
        "world_scenario",
        "first_mes",
        "greeting",
        "first_message",
        "char_greeting",
        "alternate_greetings",
        "alternate_greeting",
        "mes_example",
        "example_dialogue",
        "example_dialog",
        "example_messages",
        "system_prompt",
        "system",
        "creator_notes",
        "creator_note",
        "tags",
    ];
    let has_recognized_field = recognized_fields.iter().any(|key| data.contains_key(*key));
    let is_recognized_shape = root
        .get("spec")
        .and_then(Value::as_str)
        .is_some_and(|spec| spec.to_ascii_lowercase().contains("chara"))
        || root.get("character").is_some_and(Value::is_object);
    if !has_recognized_field && !is_recognized_shape {
        return Err(CharacterCardImportError::NoSupportedData);
    }

    let mut warnings = Vec::new();
    let name = optional_text_field(
        data,
        &["name", "char_name"],
        "Character name",
        MAX_NAME_LENGTH,
        &mut warnings,
    )
    .unwrap_or_else(|| {
        warnings
            .push("Character name was missing, so NyxAI used \"Imported Character\".".to_owned());
        "Imported Character".to_owned()
    });
    let description = optional_text_field(
        data,
        &["description"],
        "Description",
        MAX_TEXT_LENGTH,
        &mut warnings,
    )
    .unwrap_or_default();
    let personality = optional_text_field(
        data,
        &["personality", "char_persona"],
        "Personality",
        MAX_TEXT_LENGTH,
        &mut warnings,
    )
    .unwrap_or_default();
    let scenario = optional_text_field(
        data,
        &["scenario", "world_scenario"],
        "Scenario",
        MAX_TEXT_LENGTH,
        &mut warnings,
    )
    .unwrap_or_default();
    let first_message = optional_text_field(
        data,
        &["first_mes", "greeting", "first_message", "char_greeting"],
        "First message",
        MAX_TEXT_LENGTH,
        &mut warnings,
    )
    .unwrap_or_default();
    let example_dialogue = optional_text_field(
        data,
        &[
            "mes_example",
            "example_dialogue",
            "example_dialog",
            "example_messages",
        ],
        "Example dialogue",
        MAX_TEXT_LENGTH,
        &mut warnings,
    )
    .unwrap_or_default();
    let system_prompt = optional_text_field(
        data,
        &["system_prompt", "system"],
        "System prompt",
        MAX_TEXT_LENGTH,
        &mut warnings,
    )
    .unwrap_or_default();
    let creator_notes = optional_text_field(
        data,
        &["creator_notes", "creator_note"],
        "Creator notes",
        MAX_TEXT_LENGTH,
        &mut warnings,
    )
    .unwrap_or_default();
    let alternate_greetings = optional_string_collection(
        data,
        &["alternate_greetings", "alternate_greeting"],
        "alternate greeting",
        MAX_ALTERNATE_GREETINGS,
        MAX_TEXT_LENGTH,
        &mut warnings,
    );
    let tags = optional_tags(data, &mut warnings);

    let missing = [
        ("description", description.is_empty()),
        ("personality", personality.is_empty()),
        ("scenario", scenario.is_empty()),
        ("first message", first_message.is_empty()),
        ("example dialogue", example_dialogue.is_empty()),
        ("system prompt", system_prompt.is_empty()),
        ("creator notes", creator_notes.is_empty()),
    ]
    .into_iter()
    .filter_map(|(field, is_missing)| is_missing.then_some(field))
    .collect::<Vec<_>>();
    if !missing.is_empty() {
        warnings.push(format!(
            "Some optional fields were missing or empty: {}.",
            missing.join(", ")
        ));
    }
    let unknown_fields = data
        .keys()
        .filter(|key| !recognized_fields.contains(&key.as_str()))
        .count();
    if unknown_fields > 0 {
        warnings.push(format!(
            "{unknown_fields} unsupported metadata field{} {} ignored.",
            if unknown_fields == 1 { "" } else { "s" },
            if unknown_fields == 1 { "was" } else { "were" }
        ));
    }

    let draft = CharacterDraft {
        name,
        avatar_path: None,
        description,
        personality,
        scenario,
        first_message,
        example_dialogue,
        system_prompt,
        creator_notes,
        tags,
        alternate_greetings,
    };
    let draft = draft
        .normalize_and_validate()
        .map_err(|_| CharacterCardImportError::NoSupportedData)?;
    Ok((draft, warnings))
}

fn optional_text_field(
    data: &Map<String, Value>,
    names: &[&str],
    label: &str,
    maximum_length: usize,
    warnings: &mut Vec<String>,
) -> Option<String> {
    let values = names
        .iter()
        .filter_map(|name| data.get(*name))
        .collect::<Vec<_>>();
    let text = values.iter().find_map(|value| value.as_str());
    if let Some(text) = text {
        return Some(bound_text(text, label, maximum_length, warnings));
    }
    if values.iter().any(|value| !value.is_null()) {
        warnings.push(format!(
            "{label} had an unsupported value and was left blank."
        ));
    }
    None
}

fn optional_string_collection(
    data: &Map<String, Value>,
    names: &[&str],
    label: &str,
    maximum_items: usize,
    maximum_length: usize,
    warnings: &mut Vec<String>,
) -> Vec<String> {
    let Some(value) = names.iter().find_map(|name| data.get(*name)) else {
        return Vec::new();
    };
    let values: Vec<&Value> = match value {
        Value::String(_) => vec![value],
        Value::Array(values) => values.iter().collect(),
        Value::Null => return Vec::new(),
        _ => {
            warnings.push(format!(
                "{label} data had an unsupported value and was ignored."
            ));
            return Vec::new();
        }
    };
    let mut normalized = Vec::new();
    let mut discarded = 0;
    for value in values {
        if let Some(text) = value.as_str() {
            normalized.push(bound_text(text, label, maximum_length, warnings));
        } else if !value.is_null() {
            discarded += 1;
        }
    }
    if discarded > 0 {
        warnings.push(format!(
            "{discarded} non-text {label} entr{} {} ignored.",
            if discarded == 1 { "y" } else { "ies" },
            if discarded == 1 { "was" } else { "were" }
        ));
    }
    if normalized.len() > maximum_items {
        normalized.truncate(maximum_items);
        warnings.push(format!(
            "Only the first {maximum_items} {label}s were imported."
        ));
    }
    normalized
}

fn optional_tags(data: &Map<String, Value>, warnings: &mut Vec<String>) -> Vec<String> {
    let Some(value) = data.get("tags") else {
        return Vec::new();
    };
    let mut tags = match value {
        Value::String(tags) => tags.split(',').collect::<Vec<_>>(),
        Value::Array(values) => values.iter().filter_map(Value::as_str).collect(),
        Value::Null => return Vec::new(),
        _ => {
            warnings.push("Tags had an unsupported value and were left blank.".to_owned());
            return Vec::new();
        }
    }
    .into_iter()
    .map(|tag| bound_text(tag, "Tag", MAX_TAG_LENGTH, warnings))
    .collect::<Vec<_>>();
    if let Value::Array(values) = value {
        let discarded = values
            .iter()
            .filter(|value| !value.is_string() && !value.is_null())
            .count();
        if discarded > 0 {
            warnings.push(format!(
                "{discarded} non-text tag{} {} ignored.",
                if discarded == 1 { "" } else { "s" },
                if discarded == 1 { "was" } else { "were" }
            ));
        }
    }
    if tags.len() > MAX_TAGS {
        tags.truncate(MAX_TAGS);
        warnings.push(format!("Only the first {MAX_TAGS} tags were imported."));
    }
    tags
}

fn bound_text(
    text: &str,
    label: &str,
    maximum_length: usize,
    warnings: &mut Vec<String>,
) -> String {
    if text.chars().count() <= maximum_length {
        return text.to_owned();
    }
    warnings.push(format!("{label} was shortened to fit NyxAI's limit."));
    text.chars().take(maximum_length).collect()
}

fn extract_png_card_metadata(bytes: &[u8]) -> Result<Vec<u8>, CharacterCardImportError> {
    let mut offset = PNG_SIGNATURE.len();
    while offset.checked_add(12).is_some_and(|end| end <= bytes.len()) {
        let chunk_length =
            u32::from_be_bytes(bytes[offset..offset + 4].try_into().expect("slice length"))
                as usize;
        let Some(chunk_end) = offset.checked_add(12 + chunk_length) else {
            return Err(CharacterCardImportError::MissingPngMetadata);
        };
        if chunk_end > bytes.len() {
            return Err(CharacterCardImportError::MissingPngMetadata);
        }
        let chunk_type = &bytes[offset + 4..offset + 8];
        let data_start = offset + 8;
        let data_end = data_start + chunk_length;
        let data = &bytes[data_start..data_end];

        if chunk_type == b"tEXt" {
            if let Some((keyword, text)) = split_png_text(data) {
                if keyword == b"chara" {
                    if text.len() > MAX_METADATA_BYTES {
                        return Err(CharacterCardImportError::MissingPngMetadata);
                    }
                    return decode_card_metadata(text);
                }
            }
        } else if chunk_type == b"iTXt" {
            if let Some(text) = uncompressed_itxt_card_metadata(data) {
                if text.len() > MAX_METADATA_BYTES {
                    return Err(CharacterCardImportError::MissingPngMetadata);
                }
                return decode_card_metadata(text);
            }
        }
        offset = data_end + 4; // CRC is intentionally not trusted or interpreted.
    }
    Err(CharacterCardImportError::MissingPngMetadata)
}

fn split_png_text(data: &[u8]) -> Option<(&[u8], &[u8])> {
    let separator = data.iter().position(|byte| *byte == 0)?;
    Some((&data[..separator], &data[separator + 1..]))
}

fn uncompressed_itxt_card_metadata(data: &[u8]) -> Option<&[u8]> {
    let (keyword, remainder) = split_png_text(data)?;
    if keyword != b"chara" || remainder.len() < 2 || remainder[0] != 0 {
        return None;
    }
    let (_, remainder) = split_png_text(&remainder[2..])?; // language tag
    let (_, text) = split_png_text(remainder)?; // translated keyword
    Some(text)
}

fn decode_card_metadata(metadata: &[u8]) -> Result<Vec<u8>, CharacterCardImportError> {
    let metadata =
        std::str::from_utf8(metadata).map_err(|_| CharacterCardImportError::MissingPngMetadata)?;
    let metadata = metadata.trim();
    if metadata.len() > MAX_METADATA_BYTES {
        return Err(CharacterCardImportError::MissingPngMetadata);
    }
    if metadata.starts_with('{') {
        return Ok(metadata.as_bytes().to_vec());
    }
    let decoded = STANDARD
        .decode(metadata)
        .map_err(|_| CharacterCardImportError::MissingPngMetadata)?;
    if decoded.len() > MAX_METADATA_BYTES {
        return Err(CharacterCardImportError::MissingPngMetadata);
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_character_card_v2_fields_and_greetings() {
        let card = br#"{
            "spec":"chara_card_v2",
            "data": {
                "name":"Nyx",
                "description":"Archive keeper",
                "personality":"Calm",
                "scenario":"Moonlit stacks",
                "first_mes":"Welcome.",
                "alternate_greetings":["Night falls.","The archive waits.","You returned."],
                "mes_example":"Nyx: Hush.",
                "system_prompt":"Stay in character.",
                "creator_notes":"Imported for testing.",
                "tags":["fantasy","companion"],
                "unknown_field":{"ignored":true}
            }
        }"#;
        let imported = parse_character_card(card).expect("V2 card should parse");
        assert_eq!(imported.format, CharacterCardFormat::Json);
        assert_eq!(imported.draft.name, "Nyx");
        assert_eq!(imported.draft.alternate_greetings.len(), 3);
        assert_eq!(imported.draft.creator_notes, "Imported for testing.");
        assert_eq!(imported.draft.tags, vec!["fantasy", "companion"]);
        assert!(imported
            .warnings
            .iter()
            .any(|warning| warning.contains("unsupported metadata")));
    }

    #[test]
    fn imports_a_card_with_only_a_name() {
        let imported =
            parse_character_card(br#"{"name":"Nyx"}"#).expect("name-only card should import");

        assert_eq!(imported.draft.name, "Nyx");
        assert!(imported.draft.description.is_empty());
        assert!(imported.draft.first_message.is_empty());
        assert!(imported.draft.alternate_greetings.is_empty());
        assert!(!imported.warnings.is_empty());
    }

    #[test]
    fn imports_partial_cards_without_a_name_using_a_safe_fallback() {
        let imported =
            parse_character_card(br#"{"description":"An archive keeper","first_mes":"Welcome."}"#)
                .expect("recognizable partial card should import");

        assert_eq!(imported.draft.name, "Imported Character");
        assert_eq!(imported.draft.description, "An archive keeper");
        assert_eq!(imported.draft.first_message, "Welcome.");
        assert!(imported
            .warnings
            .iter()
            .any(|warning| warning.contains("name was missing")));
    }

    #[test]
    fn tolerates_nulls_single_strings_unknown_fields_and_bad_optional_values() {
        let imported = parse_character_card(
            br#"{
                "name":"Nyx",
                "description":null,
                "tags":"fantasy",
                "alternate_greetings":"Another opening.",
                "system_prompt":{"not":"text"},
                "unknown":{"extra":true}
            }"#,
        )
        .expect("optional field problems should not block import");

        assert!(imported.draft.description.is_empty());
        assert_eq!(imported.draft.tags, vec!["fantasy"]);
        assert_eq!(imported.draft.alternate_greetings, vec!["Another opening."]);
        assert!(imported.draft.system_prompt.is_empty());
        assert!(imported
            .warnings
            .iter()
            .any(|warning| warning.contains("System prompt had an unsupported value")));
    }

    #[test]
    fn discards_malformed_collection_entries_without_rejecting_the_card() {
        let imported = parse_character_card(
            br#"{
                "name":"Nyx",
                "tags":["fantasy",42,null],
                "alternate_greetings":["Hello",false,null,"Welcome back"]
            }"#,
        )
        .expect("mixed optional collections should import");

        assert_eq!(imported.draft.tags, vec!["fantasy"]);
        assert_eq!(
            imported.draft.alternate_greetings,
            vec!["Hello", "Welcome back"]
        );
        assert!(imported
            .warnings
            .iter()
            .any(|warning| warning.contains("non-text")));
    }

    #[test]
    fn accepts_a_recognizable_empty_v2_shape() {
        let imported = parse_character_card(br#"{"spec":"chara_card_v2","data":{}}"#)
            .expect("recognized card shape should use defaults");

        assert_eq!(imported.draft.name, "Imported Character");
    }

    #[test]
    fn rejects_only_unreadable_or_unrelated_json() {
        assert!(matches!(
            parse_character_card(b"{"),
            Err(CharacterCardImportError::InvalidJson)
        ));
        assert!(matches!(
            parse_character_card(br#"{"unrelated":true}"#),
            Err(CharacterCardImportError::NoSupportedData)
        ));
        assert!(matches!(
            parse_character_card(b"not a card"),
            Err(CharacterCardImportError::UnsupportedFormat)
        ));
    }

    #[test]
    fn extracts_base64_metadata_from_png_character_cards() {
        let card = br#"{"name":"Nyx","first_mes":"Welcome.","alternate_greetings":["Alternate."]}"#;
        let encoded = STANDARD.encode(card);
        let mut png = PNG_SIGNATURE.to_vec();
        let payload = [b"chara\0".as_slice(), encoded.as_bytes()].concat();
        png.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        png.extend_from_slice(b"tEXt");
        png.extend_from_slice(&payload);
        png.extend_from_slice(&[0, 0, 0, 0]);

        let imported = parse_character_card(&png).expect("PNG card should parse");
        assert_eq!(imported.format, CharacterCardFormat::Png);
        assert_eq!(imported.draft.alternate_greetings, vec!["Alternate."]);
        assert_eq!(imported.avatar_bytes.as_deref(), Some(png.as_slice()));
    }

    #[test]
    fn rejects_png_without_supported_card_metadata() {
        assert!(matches!(
            parse_character_card(PNG_SIGNATURE),
            Err(CharacterCardImportError::MissingPngMetadata)
        ));
    }
}
