use anyhow::{bail, Context};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::domain::{
    chat::PersistedMessage,
    memory::{normalized_memory_key, MemoryDraft, MemoryEntry, MemoryScope},
};

#[derive(FromRow)]
struct MemoryRecord {
    id: String,
    character_id: String,
    chat_id: Option<String>,
    scope: String,
    content: String,
    importance: i64,
    manually_created: bool,
    source_start_message_id: Option<String>,
    source_end_message_id: Option<String>,
    embedding_model: Option<String>,
    embedding_dimension: Option<i64>,
    created_at: String,
    updated_at: String,
}

#[derive(FromRow)]
struct MemoryVectorRecord {
    id: String,
    character_id: String,
    chat_id: Option<String>,
    scope: String,
    content: String,
    importance: i64,
    manually_created: bool,
    source_start_message_id: Option<String>,
    source_end_message_id: Option<String>,
    embedding_model: String,
    embedding_dimension: i64,
    vector_blob: Vec<u8>,
    created_at: String,
    updated_at: String,
}

#[derive(Clone, Debug)]
pub struct MemoryVectorCandidate {
    pub entry: MemoryEntry,
    pub vector: Vec<f32>,
}

pub async fn list_memories(
    pool: &SqlitePool,
    character_id: Option<&str>,
    chat_id: Option<&str>,
    search: Option<&str>,
) -> anyhow::Result<Vec<MemoryEntry>> {
    if let Some(character_id) = character_id {
        validate_id(character_id, "Character")?;
    }
    if let Some(chat_id) = chat_id {
        validate_id(chat_id, "Chat")?;
    }
    let mut sql = "SELECT m.id, m.character_id, m.chat_id, m.scope, m.content, m.importance, m.manually_created, m.source_start_message_id, m.source_end_message_id, v.embedding_model, v.embedding_dimension, m.created_at, m.updated_at FROM memory_entries m LEFT JOIN memory_vectors v ON v.memory_id = m.id WHERE 1=1".to_owned();
    if character_id.is_some() {
        sql.push_str(" AND m.character_id = ?");
    }
    if chat_id.is_some() {
        sql.push_str(" AND m.chat_id = ?");
    }
    if search.is_some_and(|value| !value.trim().is_empty()) {
        sql.push_str(" AND LOWER(m.content) LIKE ?");
    }
    sql.push_str(" ORDER BY m.importance DESC, m.updated_at DESC, m.id");
    let mut query = sqlx::query_as::<_, MemoryRecord>(&sql);
    if let Some(character_id) = character_id {
        query = query.bind(character_id);
    }
    if let Some(chat_id) = chat_id {
        query = query.bind(chat_id);
    }
    if let Some(search) = search.filter(|value| !value.trim().is_empty()) {
        query = query.bind(format!("%{}%", search.trim().to_lowercase()));
    }
    query
        .fetch_all(pool)
        .await
        .context("could not load memories")?
        .into_iter()
        .map(memory_from_record)
        .collect()
}

pub async fn get_memory(pool: &SqlitePool, memory_id: &str) -> anyhow::Result<Option<MemoryEntry>> {
    validate_id(memory_id, "Memory")?;
    sqlx::query_as::<_, MemoryRecord>("SELECT m.id, m.character_id, m.chat_id, m.scope, m.content, m.importance, m.manually_created, m.source_start_message_id, m.source_end_message_id, v.embedding_model, v.embedding_dimension, m.created_at, m.updated_at FROM memory_entries m LEFT JOIN memory_vectors v ON v.memory_id = m.id WHERE m.id = ?")
        .bind(memory_id).fetch_optional(pool).await.context("could not load memory")?.map(memory_from_record).transpose()
}

pub async fn create_memory_with_vector(
    pool: &SqlitePool,
    character_id: &str,
    draft: MemoryDraft,
    manually_created: bool,
    embedding_model: &str,
    vector: &[f32],
) -> anyhow::Result<MemoryEntry> {
    validate_id(character_id, "Character")?;
    let draft = draft.normalize_and_validate()?;
    validate_scope(pool, character_id, &draft).await?;
    validate_vector(vector)?;
    let normalized = normalized_memory_key(&draft.content);
    if memory_by_normalized(pool, character_id, &draft, &normalized)
        .await?
        .is_some()
    {
        bail!("That memory already exists in this scope.");
    }
    let id = Uuid::new_v4().to_string();
    let mut transaction = pool.begin().await.context("could not start memory save")?;
    sqlx::query("INSERT INTO memory_entries (id, character_id, chat_id, scope, content, content_normalized, importance, manually_created, source_start_message_id, source_end_message_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&id).bind(character_id).bind(&draft.chat_id).bind(scope_value(&draft.scope)).bind(&draft.content).bind(&normalized).bind(draft.importance).bind(manually_created).bind(&draft.source_start_message_id).bind(&draft.source_end_message_id).execute(&mut *transaction).await.context("could not create memory")?;
    write_vector(&mut transaction, &id, embedding_model, vector).await?;
    transaction
        .commit()
        .await
        .context("could not finish memory save")?;
    get_memory(pool, &id)
        .await?
        .context("created memory could not be reloaded")
}

/// The caller embeds before this transaction. Therefore an embedding failure
/// never replaces a working memory/vector pair with inconsistent text.
pub async fn update_memory_with_vector(
    pool: &SqlitePool,
    memory_id: &str,
    draft: MemoryDraft,
    embedding_model: &str,
    vector: &[f32],
) -> anyhow::Result<Option<MemoryEntry>> {
    validate_id(memory_id, "Memory")?;
    let Some(existing) = get_memory(pool, memory_id).await? else {
        return Ok(None);
    };
    let draft = draft.normalize_and_validate()?;
    validate_scope(pool, &existing.character_id, &draft).await?;
    validate_vector(vector)?;
    let normalized = normalized_memory_key(&draft.content);
    if let Some(duplicate) =
        memory_by_normalized(pool, &existing.character_id, &draft, &normalized).await?
    {
        if duplicate != memory_id {
            bail!("That memory already exists in this scope.");
        }
    }
    let mut transaction = pool
        .begin()
        .await
        .context("could not start memory update")?;
    sqlx::query("UPDATE memory_entries SET chat_id = ?, scope = ?, content = ?, content_normalized = ?, importance = ?, source_start_message_id = ?, source_end_message_id = ?, updated_at = STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(&draft.chat_id).bind(scope_value(&draft.scope)).bind(&draft.content).bind(&normalized).bind(draft.importance).bind(&draft.source_start_message_id).bind(&draft.source_end_message_id).bind(memory_id).execute(&mut *transaction).await.context("could not update memory")?;
    write_vector(&mut transaction, memory_id, embedding_model, vector).await?;
    transaction
        .commit()
        .await
        .context("could not finish memory update")?;
    get_memory(pool, memory_id).await
}

pub async fn delete_memory(
    pool: &SqlitePool,
    memory_id: &str,
) -> anyhow::Result<Option<MemoryEntry>> {
    let memory = get_memory(pool, memory_id).await?;
    if memory.is_some() {
        sqlx::query("DELETE FROM memory_entries WHERE id = ?")
            .bind(memory_id)
            .execute(pool)
            .await
            .context("could not delete memory")?;
    }
    Ok(memory)
}

pub async fn memory_vectors_for_context(
    pool: &SqlitePool,
    character_id: &str,
    chat_id: &str,
    embedding_model: &str,
) -> anyhow::Result<Vec<MemoryVectorCandidate>> {
    validate_id(character_id, "Character")?;
    validate_id(chat_id, "Chat")?;
    let rows = sqlx::query_as::<_, MemoryVectorRecord>("SELECT m.id, m.character_id, m.chat_id, m.scope, m.content, m.importance, m.manually_created, m.source_start_message_id, m.source_end_message_id, v.embedding_model, v.embedding_dimension, v.vector_blob, m.created_at, m.updated_at FROM memory_entries m JOIN memory_vectors v ON v.memory_id = m.id WHERE m.character_id = ? AND v.embedding_model = ? AND (m.scope = 'character' OR (m.scope = 'chat' AND m.chat_id = ?))")
        .bind(character_id).bind(embedding_model).bind(chat_id).fetch_all(pool).await.context("could not load memory vectors")?;
    Ok(rows
        .into_iter()
        .filter_map(|record| {
            let decoded =
                decode_vector(&record.vector_blob, record.embedding_dimension as usize).ok()?;
            let scope = match record.scope.as_str() {
                "chat" => MemoryScope::Chat,
                "character" => MemoryScope::Character,
                _ => return None,
            };
            Some(MemoryVectorCandidate {
                entry: MemoryEntry {
                    id: record.id,
                    character_id: record.character_id,
                    chat_id: record.chat_id,
                    scope,
                    content: record.content,
                    importance: record.importance,
                    manually_created: record.manually_created,
                    source_start_message_id: record.source_start_message_id,
                    source_end_message_id: record.source_end_message_id,
                    embedding_model: Some(record.embedding_model),
                    embedding_dimension: Some(record.embedding_dimension),
                    created_at: record.created_at,
                    updated_at: record.updated_at,
                },
                vector: decoded,
            })
        })
        .collect())
}

pub async fn list_memories_without_current_vector(
    pool: &SqlitePool,
    model: &str,
) -> anyhow::Result<Vec<MemoryEntry>> {
    sqlx::query_as::<_, MemoryRecord>("SELECT m.id, m.character_id, m.chat_id, m.scope, m.content, m.importance, m.manually_created, m.source_start_message_id, m.source_end_message_id, v.embedding_model, v.embedding_dimension, m.created_at, m.updated_at FROM memory_entries m LEFT JOIN memory_vectors v ON v.memory_id = m.id WHERE v.embedding_model IS NULL OR v.embedding_model != ? ORDER BY m.updated_at ASC")
        .bind(model).fetch_all(pool).await.context("could not load memories needing reindex")?.into_iter().map(memory_from_record).collect()
}

pub async fn replace_memory_vector(
    pool: &SqlitePool,
    memory_id: &str,
    model: &str,
    vector: &[f32],
) -> anyhow::Result<()> {
    validate_id(memory_id, "Memory")?;
    validate_vector(vector)?;
    let mut transaction = pool
        .begin()
        .await
        .context("could not start vector update")?;
    write_vector(&mut transaction, memory_id, model, vector).await?;
    transaction
        .commit()
        .await
        .context("could not finish vector update")
}

pub async fn count_memories(pool: &SqlitePool) -> anyhow::Result<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM memory_entries")
        .fetch_one(pool)
        .await
        .context("could not count memories")
}

pub async fn unprocessed_messages(
    pool: &SqlitePool,
    chat_id: &str,
) -> anyhow::Result<Vec<PersistedMessage>> {
    validate_id(chat_id, "Chat")?;
    let last = sqlx::query_scalar::<_, Option<String>>(
        "SELECT last_processed_message_id FROM memory_chat_state WHERE chat_id = ?",
    )
    .bind(chat_id)
    .fetch_optional(pool)
    .await
    .context("could not load memory progress")?
    .flatten();
    let messages = crate::storage::list_chat_messages(pool, chat_id).await?;
    let start = last
        .as_deref()
        .and_then(|id| {
            messages
                .iter()
                .position(|message| message.id == id)
                .map(|index| index + 1)
        })
        .unwrap_or(0);
    Ok(messages.into_iter().skip(start).collect())
}

pub async fn mark_messages_processed(
    pool: &SqlitePool,
    chat_id: &str,
    message_id: &str,
) -> anyhow::Result<()> {
    validate_id(chat_id, "Chat")?;
    validate_id(message_id, "Message")?;
    sqlx::query("INSERT INTO memory_chat_state (chat_id, last_processed_message_id) VALUES (?, ?) ON CONFLICT(chat_id) DO UPDATE SET last_processed_message_id = excluded.last_processed_message_id, updated_at = STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')")
        .bind(chat_id).bind(message_id).execute(pool).await.context("could not update memory progress")?;
    Ok(())
}

pub async fn record_message_memories(
    pool: &SqlitePool,
    message_id: &str,
    memory_ids: &[String],
) -> anyhow::Result<()> {
    validate_id(message_id, "Message")?;
    for memory_id in memory_ids {
        validate_id(memory_id, "Memory")?;
        sqlx::query(
            "INSERT OR IGNORE INTO message_memory_entries (message_id, memory_id) VALUES (?, ?)",
        )
        .bind(message_id)
        .bind(memory_id)
        .execute(pool)
        .await
        .context("could not record memory context")?;
    }
    Ok(())
}

pub async fn list_message_memories(
    pool: &SqlitePool,
    message_id: &str,
) -> anyhow::Result<Vec<MemoryEntry>> {
    validate_id(message_id, "Message")?;
    sqlx::query_as::<_, MemoryRecord>("SELECT m.id, m.character_id, m.chat_id, m.scope, m.content, m.importance, m.manually_created, m.source_start_message_id, m.source_end_message_id, v.embedding_model, v.embedding_dimension, m.created_at, m.updated_at FROM memory_entries m LEFT JOIN memory_vectors v ON v.memory_id = m.id JOIN message_memory_entries mm ON mm.memory_id = m.id WHERE mm.message_id = ? ORDER BY m.importance DESC, m.id")
        .bind(message_id).fetch_all(pool).await.context("could not load memory context")?.into_iter().map(memory_from_record).collect()
}

fn memory_from_record(record: MemoryRecord) -> anyhow::Result<MemoryEntry> {
    Ok(MemoryEntry {
        id: record.id,
        character_id: record.character_id,
        chat_id: record.chat_id,
        scope: match record.scope.as_str() {
            "chat" => MemoryScope::Chat,
            "character" => MemoryScope::Character,
            _ => bail!("stored memory scope is invalid"),
        },
        content: record.content,
        importance: record.importance,
        manually_created: record.manually_created,
        source_start_message_id: record.source_start_message_id,
        source_end_message_id: record.source_end_message_id,
        embedding_model: record.embedding_model,
        embedding_dimension: record.embedding_dimension,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}
fn scope_value(scope: &MemoryScope) -> &'static str {
    match scope {
        MemoryScope::Chat => "chat",
        MemoryScope::Character => "character",
    }
}
fn encode_vector(vector: &[f32]) -> Vec<u8> {
    vector
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}
fn decode_vector(blob: &[u8], dimension: usize) -> anyhow::Result<Vec<f32>> {
    if dimension == 0 || blob.len() != dimension * 4 {
        bail!("stored memory vector has an invalid dimension")
    };
    let vector = blob
        .chunks_exact(4)
        .map(|bytes| f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .collect::<Vec<_>>();
    validate_vector(&vector)?;
    Ok(vector)
}
fn validate_vector(vector: &[f32]) -> anyhow::Result<()> {
    if vector.is_empty() || vector.len() > 65_536 || !vector.iter().all(|value| value.is_finite()) {
        bail!("embedding vector is invalid")
    };
    Ok(())
}
async fn write_vector(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    memory_id: &str,
    model: &str,
    vector: &[f32],
) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO memory_vectors (memory_id, embedding_model, embedding_dimension, vector_blob) VALUES (?, ?, ?, ?) ON CONFLICT(memory_id) DO UPDATE SET embedding_model = excluded.embedding_model, embedding_dimension = excluded.embedding_dimension, vector_blob = excluded.vector_blob, updated_at = STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')").bind(memory_id).bind(model).bind(vector.len() as i64).bind(encode_vector(vector)).execute(&mut **transaction).await.context("could not save memory vector")?;
    Ok(())
}
async fn validate_scope(
    pool: &SqlitePool,
    character_id: &str,
    draft: &MemoryDraft,
) -> anyhow::Result<()> {
    if let MemoryScope::Chat = draft.scope {
        let chat_id = draft
            .chat_id
            .as_deref()
            .context("Choose a chat-scoped memory location.")?;
        validate_id(chat_id, "Chat")?;
        let belongs =
            sqlx::query_scalar::<_, i64>("SELECT 1 FROM chats WHERE id = ? AND character_id = ?")
                .bind(chat_id)
                .bind(character_id)
                .fetch_optional(pool)
                .await?
                .is_some();
        if !belongs {
            bail!("That chat does not belong to this character.");
        }
    }
    Ok(())
}
async fn memory_by_normalized(
    pool: &SqlitePool,
    character_id: &str,
    draft: &MemoryDraft,
    normalized: &str,
) -> anyhow::Result<Option<String>> {
    sqlx::query_scalar("SELECT id FROM memory_entries WHERE character_id = ? AND scope = ? AND IFNULL(chat_id, '') = IFNULL(?, '') AND content_normalized = ?").bind(character_id).bind(scope_value(&draft.scope)).bind(&draft.chat_id).bind(normalized).fetch_optional(pool).await.context("could not check duplicate memory")
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
        storage::{create_character, create_chat, delete_chat},
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
    async fn memories_stay_with_their_chat_and_cascade_cleanly() {
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
        let first = create_chat(&pool, &character.id, "First", None, None)
            .await
            .expect("chat query")
            .expect("chat should create");
        let second = create_chat(&pool, &character.id, "Second", None, None)
            .await
            .expect("chat query")
            .expect("chat should create");
        let vector = vec![0.1, 0.2, 0.3];
        let memory = create_memory_with_vector(
            &pool,
            &character.id,
            MemoryDraft {
                chat_id: Some(first.id.clone()),
                content: "Nyx promised to meet at dawn.".to_owned(),
                importance: 80,
                ..MemoryDraft::default()
            },
            false,
            "local-embed",
            &vector,
        )
        .await
        .expect("memory should create");
        assert_eq!(
            memory_vectors_for_context(&pool, &character.id, &first.id, "local-embed")
                .await
                .expect("first chat memories")
                .len(),
            1
        );
        assert!(
            memory_vectors_for_context(&pool, &character.id, &second.id, "local-embed")
                .await
                .expect("second chat memories")
                .is_empty()
        );
        delete_chat(&pool, &character.id, &first.id)
            .await
            .expect("chat should delete");
        assert!(get_memory(&pool, &memory.id)
            .await
            .expect("memory lookup")
            .is_none());
    }
}
