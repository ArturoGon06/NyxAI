use anyhow::{bail, Context};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::domain::lorebook::{
    LoreMatchMode, Lorebook, LorebookDraft, LorebookEntry, LorebookEntryDraft,
};

#[derive(FromRow)]
struct LorebookRecord {
    id: String,
    name: String,
    description: String,
    enabled: bool,
    created_at: String,
    updated_at: String,
}
#[derive(FromRow)]
struct EntryRecord {
    id: String,
    lorebook_id: String,
    name: String,
    keys_json: String,
    content: String,
    enabled: bool,
    priority: i64,
    case_sensitive: bool,
    match_mode: String,
    always_active: bool,
    created_at: String,
    updated_at: String,
}

pub async fn list_lorebooks(pool: &SqlitePool) -> anyhow::Result<Vec<Lorebook>> {
    sqlx::query_as::<_, LorebookRecord>("SELECT id, name, description, enabled, created_at, updated_at FROM lorebooks ORDER BY updated_at DESC, name COLLATE NOCASE")
        .fetch_all(pool).await.context("could not load lorebooks")?.into_iter().map(lorebook_from_record).collect()
}
pub async fn get_lorebook(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Lorebook>> {
    validate_id(id, "Lorebook")?;
    sqlx::query_as::<_, LorebookRecord>(
        "SELECT id, name, description, enabled, created_at, updated_at FROM lorebooks WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("could not load lorebook")?
    .map(lorebook_from_record)
    .transpose()
}
pub async fn create_lorebook(pool: &SqlitePool, draft: LorebookDraft) -> anyhow::Result<Lorebook> {
    let draft = draft.normalize_and_validate()?;
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO lorebooks (id, name, description, enabled) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(draft.name)
        .bind(draft.description)
        .bind(draft.enabled)
        .execute(pool)
        .await
        .context("could not create lorebook")?;
    get_lorebook(pool, &id)
        .await?
        .context("created lorebook could not be reloaded")
}
pub async fn update_lorebook(
    pool: &SqlitePool,
    id: &str,
    draft: LorebookDraft,
) -> anyhow::Result<Option<Lorebook>> {
    validate_id(id, "Lorebook")?;
    let draft = draft.normalize_and_validate()?;
    let result = sqlx::query("UPDATE lorebooks SET name = ?, description = ?, enabled = ?, updated_at = STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?").bind(draft.name).bind(draft.description).bind(draft.enabled).bind(id).execute(pool).await.context("could not update lorebook")?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    get_lorebook(pool, id).await
}
pub async fn delete_lorebook(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Lorebook>> {
    let book = get_lorebook(pool, id).await?;
    if book.is_some() {
        sqlx::query("DELETE FROM lorebooks WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await
            .context("could not delete lorebook")?;
    }
    Ok(book)
}
pub async fn duplicate_lorebook(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Lorebook>> {
    let Some(source) = get_lorebook(pool, id).await? else {
        return Ok(None);
    };
    let entries = list_lorebook_entries(pool, id).await?;
    let copy = create_lorebook(
        pool,
        LorebookDraft {
            name: format!("{} Copy", source.name),
            description: source.description,
            enabled: source.enabled,
        },
    )
    .await?;
    for entry in entries {
        create_lorebook_entry(
            pool,
            &copy.id,
            LorebookEntryDraft {
                name: entry.name,
                keys: entry.keys,
                content: entry.content,
                enabled: entry.enabled,
                priority: entry.priority,
                case_sensitive: entry.case_sensitive,
                match_mode: entry.match_mode,
                always_active: entry.always_active,
            },
        )
        .await?;
    }
    Ok(Some(copy))
}

pub async fn list_lorebook_entries(
    pool: &SqlitePool,
    lorebook_id: &str,
) -> anyhow::Result<Vec<LorebookEntry>> {
    validate_id(lorebook_id, "Lorebook")?;
    sqlx::query_as::<_, EntryRecord>("SELECT id, lorebook_id, name, keys_json, content, enabled, priority, case_sensitive, match_mode, always_active, created_at, updated_at FROM lorebook_entries WHERE lorebook_id = ? ORDER BY priority DESC, name COLLATE NOCASE, id").bind(lorebook_id).fetch_all(pool).await.context("could not load lore entries")?.into_iter().map(entry_from_record).collect()
}
pub async fn get_lorebook_entry(
    pool: &SqlitePool,
    lorebook_id: &str,
    entry_id: &str,
) -> anyhow::Result<Option<LorebookEntry>> {
    validate_id(lorebook_id, "Lorebook")?;
    validate_id(entry_id, "Lore entry")?;
    sqlx::query_as::<_, EntryRecord>("SELECT id, lorebook_id, name, keys_json, content, enabled, priority, case_sensitive, match_mode, always_active, created_at, updated_at FROM lorebook_entries WHERE id = ? AND lorebook_id = ?").bind(entry_id).bind(lorebook_id).fetch_optional(pool).await.context("could not load lore entry")?.map(entry_from_record).transpose()
}
pub async fn create_lorebook_entry(
    pool: &SqlitePool,
    lorebook_id: &str,
    draft: LorebookEntryDraft,
) -> anyhow::Result<Option<LorebookEntry>> {
    validate_id(lorebook_id, "Lorebook")?;
    let draft = draft.normalize_and_validate()?;
    if get_lorebook(pool, lorebook_id).await?.is_none() {
        return Ok(None);
    }
    let id = Uuid::new_v4().to_string();
    write_entry(pool, &id, lorebook_id, &draft, false).await?;
    get_lorebook_entry(pool, lorebook_id, &id).await
}
pub async fn update_lorebook_entry(
    pool: &SqlitePool,
    lorebook_id: &str,
    entry_id: &str,
    draft: LorebookEntryDraft,
) -> anyhow::Result<Option<LorebookEntry>> {
    validate_id(lorebook_id, "Lorebook")?;
    validate_id(entry_id, "Lore entry")?;
    let draft = draft.normalize_and_validate()?;
    if !write_entry(pool, entry_id, lorebook_id, &draft, true).await? {
        return Ok(None);
    }
    get_lorebook_entry(pool, lorebook_id, entry_id).await
}
pub async fn delete_lorebook_entry(
    pool: &SqlitePool,
    lorebook_id: &str,
    entry_id: &str,
) -> anyhow::Result<Option<LorebookEntry>> {
    let entry = get_lorebook_entry(pool, lorebook_id, entry_id).await?;
    if entry.is_some() {
        sqlx::query("DELETE FROM lorebook_entries WHERE id = ? AND lorebook_id = ?")
            .bind(entry_id)
            .bind(lorebook_id)
            .execute(pool)
            .await
            .context("could not delete lore entry")?;
    }
    Ok(entry)
}

pub async fn list_character_lorebooks(
    pool: &SqlitePool,
    character_id: &str,
) -> anyhow::Result<Vec<Lorebook>> {
    list_associated_lorebooks(pool, "character_lorebooks", "character_id", character_id).await
}
pub async fn list_chat_lorebooks(
    pool: &SqlitePool,
    chat_id: &str,
) -> anyhow::Result<Vec<Lorebook>> {
    list_associated_lorebooks(pool, "chat_lorebooks", "chat_id", chat_id).await
}
pub async fn set_character_lorebooks(
    pool: &SqlitePool,
    character_id: &str,
    lorebook_ids: &[String],
) -> anyhow::Result<()> {
    replace_associations(
        pool,
        "character_lorebooks",
        "character_id",
        character_id,
        lorebook_ids,
    )
    .await
}
pub async fn set_chat_lorebooks(
    pool: &SqlitePool,
    chat_id: &str,
    lorebook_ids: &[String],
) -> anyhow::Result<()> {
    replace_associations(pool, "chat_lorebooks", "chat_id", chat_id, lorebook_ids).await
}

/// Entries from enabled book associations are loaded with two set-based joins;
/// a chat may add books beyond its character without duplicating an entry.
pub async fn list_resolvable_lore_entries(
    pool: &SqlitePool,
    character_id: &str,
    chat_id: &str,
) -> anyhow::Result<Vec<LorebookEntry>> {
    validate_id(character_id, "Character")?;
    validate_id(chat_id, "Chat")?;
    sqlx::query_as::<_, EntryRecord>("SELECT DISTINCT e.id, e.lorebook_id, e.name, e.keys_json, e.content, e.enabled, e.priority, e.case_sensitive, e.match_mode, e.always_active, e.created_at, e.updated_at FROM lorebook_entries e JOIN lorebooks l ON l.id = e.lorebook_id WHERE l.enabled = 1 AND e.enabled = 1 AND (EXISTS (SELECT 1 FROM character_lorebooks cl WHERE cl.character_id = ? AND cl.lorebook_id = e.lorebook_id AND cl.enabled = 1) OR EXISTS (SELECT 1 FROM chat_lorebooks tl WHERE tl.chat_id = ? AND tl.lorebook_id = e.lorebook_id AND tl.enabled = 1)) ORDER BY e.priority DESC, e.name COLLATE NOCASE, e.id").bind(character_id).bind(chat_id).fetch_all(pool).await.context("could not load attached lore entries")?.into_iter().map(entry_from_record).collect()
}
pub async fn record_message_lore_entries(
    pool: &SqlitePool,
    message_id: &str,
    entry_ids: &[String],
) -> anyhow::Result<()> {
    validate_id(message_id, "Message")?;
    for entry_id in entry_ids {
        validate_id(entry_id, "Lore entry")?;
        sqlx::query("INSERT OR IGNORE INTO message_lore_entries (message_id, lorebook_entry_id) VALUES (?, ?)").bind(message_id).bind(entry_id).execute(pool).await.context("could not record applied lore")?;
    }
    Ok(())
}
pub async fn list_message_lore_entries(
    pool: &SqlitePool,
    message_id: &str,
) -> anyhow::Result<Vec<LorebookEntry>> {
    validate_id(message_id, "Message")?;
    sqlx::query_as::<_, EntryRecord>("SELECT e.id, e.lorebook_id, e.name, e.keys_json, e.content, e.enabled, e.priority, e.case_sensitive, e.match_mode, e.always_active, e.created_at, e.updated_at FROM lorebook_entries e JOIN message_lore_entries ml ON ml.lorebook_entry_id = e.id WHERE ml.message_id = ? ORDER BY e.priority DESC, e.name COLLATE NOCASE").bind(message_id).fetch_all(pool).await.context("could not load applied lore")?.into_iter().map(entry_from_record).collect()
}

async fn write_entry(
    pool: &SqlitePool,
    id: &str,
    lorebook_id: &str,
    draft: &LorebookEntryDraft,
    update: bool,
) -> anyhow::Result<bool> {
    let keys = serde_json::to_string(&draft.keys).context("could not encode lore keys")?;
    if !update {
        sqlx::query("INSERT INTO lorebook_entries (id, lorebook_id, name, keys_json, content, enabled, priority, case_sensitive, match_mode, always_active) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)").bind(id).bind(lorebook_id).bind(&draft.name).bind(keys).bind(&draft.content).bind(draft.enabled).bind(draft.priority).bind(draft.case_sensitive).bind(match_mode_value(&draft.match_mode)).bind(draft.always_active).execute(pool).await.context("could not create lore entry")?;
        return Ok(true);
    }
    let result = sqlx::query("UPDATE lorebook_entries SET name = ?, keys_json = ?, content = ?, enabled = ?, priority = ?, case_sensitive = ?, match_mode = ?, always_active = ?, updated_at = STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ? AND lorebook_id = ?").bind(&draft.name).bind(keys).bind(&draft.content).bind(draft.enabled).bind(draft.priority).bind(draft.case_sensitive).bind(match_mode_value(&draft.match_mode)).bind(draft.always_active).bind(id).bind(lorebook_id).execute(pool).await.context("could not update lore entry")?;
    Ok(result.rows_affected() > 0)
}
async fn list_associated_lorebooks(
    pool: &SqlitePool,
    table: &str,
    id_column: &str,
    owner_id: &str,
) -> anyhow::Result<Vec<Lorebook>> {
    validate_id(owner_id, "Owner")?;
    let sql = format!("SELECT l.id, l.name, l.description, l.enabled, l.created_at, l.updated_at FROM lorebooks l JOIN {table} a ON a.lorebook_id = l.id WHERE a.{id_column} = ? AND a.enabled = 1 ORDER BY l.name COLLATE NOCASE");
    sqlx::query_as::<_, LorebookRecord>(&sql)
        .bind(owner_id)
        .fetch_all(pool)
        .await
        .context("could not load lorebook associations")?
        .into_iter()
        .map(lorebook_from_record)
        .collect()
}
async fn replace_associations(
    pool: &SqlitePool,
    table: &str,
    id_column: &str,
    owner_id: &str,
    lorebook_ids: &[String],
) -> anyhow::Result<()> {
    validate_id(owner_id, "Owner")?;
    let mut unique = Vec::new();
    for id in lorebook_ids {
        validate_id(id, "Lorebook")?;
        if !unique.contains(id) {
            unique.push(id.clone());
        }
    }
    let mut transaction = pool
        .begin()
        .await
        .context("could not start lorebook association update")?;
    for id in &unique {
        let exists = sqlx::query_scalar::<_, i64>("SELECT 1 FROM lorebooks WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *transaction)
            .await?
            .is_some();
        if !exists {
            bail!("That lorebook no longer exists.");
        }
    }
    let delete = format!("DELETE FROM {table} WHERE {id_column} = ?");
    sqlx::query(&delete)
        .bind(owner_id)
        .execute(&mut *transaction)
        .await?;
    let insert =
        format!("INSERT INTO {table} ({id_column}, lorebook_id, enabled) VALUES (?, ?, 1)");
    for id in unique {
        sqlx::query(&insert)
            .bind(owner_id)
            .bind(id)
            .execute(&mut *transaction)
            .await?;
    }
    transaction
        .commit()
        .await
        .context("could not finish lorebook association update")
}
fn lorebook_from_record(record: LorebookRecord) -> anyhow::Result<Lorebook> {
    Ok(Lorebook {
        id: record.id,
        name: record.name,
        description: record.description,
        enabled: record.enabled,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}
fn entry_from_record(record: EntryRecord) -> anyhow::Result<LorebookEntry> {
    Ok(LorebookEntry {
        id: record.id,
        lorebook_id: record.lorebook_id,
        name: record.name,
        keys: serde_json::from_str(&record.keys_json).context("stored lore keys are invalid")?,
        content: record.content,
        enabled: record.enabled,
        priority: record.priority,
        case_sensitive: record.case_sensitive,
        match_mode: match record.match_mode.as_str() {
            "any_key" => LoreMatchMode::AnyKey,
            "all_keys" => LoreMatchMode::AllKeys,
            _ => bail!("stored lore match mode is invalid"),
        },
        always_active: record.always_active,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}
fn match_mode_value(mode: &LoreMatchMode) -> &'static str {
    match mode {
        LoreMatchMode::AnyKey => "any_key",
        LoreMatchMode::AllKeys => "all_keys",
    }
}
fn validate_id(id: &str, label: &str) -> anyhow::Result<()> {
    if Uuid::parse_str(id).is_err() {
        bail!("{label} ID is invalid.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;
    use crate::{
        domain::character::CharacterDraft,
        storage::{append_assistant_message, create_character, create_chat},
    };

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test database should connect");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations should apply");
        pool
    }

    #[tokio::test]
    async fn attachments_are_isolated_and_message_context_is_persisted() {
        let pool = test_pool().await;
        let character = create_character(
            &pool,
            CharacterDraft {
                name: "Nyx".to_owned(),
                ..CharacterDraft::default()
            },
        )
        .await
        .expect("character should create");
        let chat = create_chat(&pool, &character.id, "Main", None, None)
            .await
            .expect("chat should create")
            .expect("character should exist");

        let character_book = create_lorebook(
            &pool,
            LorebookDraft {
                name: "Elderglen".to_owned(),
                ..LorebookDraft::default()
            },
        )
        .await
        .expect("book should create");
        let chat_book = create_lorebook(
            &pool,
            LorebookDraft {
                name: "Private scene".to_owned(),
                ..LorebookDraft::default()
            },
        )
        .await
        .expect("book should create");
        let village = create_lorebook_entry(
            &pool,
            &character_book.id,
            LorebookEntryDraft {
                name: "Blackwood".to_owned(),
                keys: vec!["blackwood".to_owned()],
                content: "A northern settlement.".to_owned(),
                priority: 100,
                ..LorebookEntryDraft::default()
            },
        )
        .await
        .expect("entry should create")
        .expect("book should exist");
        let rule = create_lorebook_entry(
            &pool,
            &chat_book.id,
            LorebookEntryDraft {
                name: "Private rule".to_owned(),
                content: "This applies only here.".to_owned(),
                always_active: true,
                ..LorebookEntryDraft::default()
            },
        )
        .await
        .expect("entry should create")
        .expect("book should exist");

        set_character_lorebooks(
            &pool,
            &character.id,
            std::slice::from_ref(&character_book.id),
        )
        .await
        .expect("character association should save");
        set_chat_lorebooks(&pool, &chat.id, std::slice::from_ref(&chat_book.id))
            .await
            .expect("chat association should save");
        let entries = list_resolvable_lore_entries(&pool, &character.id, &chat.id)
            .await
            .expect("attached entries should load");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, village.id);
        assert!(entries.iter().any(|entry| entry.id == rule.id));

        let message = append_assistant_message(
            &pool,
            &chat.id,
            "A response using lore.",
            Some("a0000000-0000-4000-8000-000000000099"),
        )
        .await
        .expect("message should create");
        record_message_lore_entries(&pool, &message.id, &[village.id.clone(), rule.id.clone()])
            .await
            .expect("context should persist");
        assert_eq!(
            list_message_lore_entries(&pool, &message.id)
                .await
                .expect("context should load")
                .len(),
            2
        );

        let duplicate = duplicate_lorebook(&pool, &character_book.id)
            .await
            .expect("book should duplicate")
            .expect("source should exist");
        assert_ne!(duplicate.id, character_book.id);
        assert_eq!(
            list_lorebook_entries(&pool, &duplicate.id)
                .await
                .expect("duplicate entries should load")
                .len(),
            1
        );
    }
}
