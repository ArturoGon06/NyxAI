use anyhow::{bail, Context};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::domain::{
    chat::{validate_persisted_message, Chat, PersistedMessage},
    conversation::MessageRole,
};

#[derive(FromRow)]
struct ChatRecord {
    id: String,
    character_id: String,
    persona_id: Option<String>,
    title: String,
    created_at: String,
    updated_at: String,
}

#[derive(FromRow)]
struct MessageRecord {
    id: String,
    chat_id: String,
    role: String,
    content: String,
    created_at: String,
}

pub async fn list_chats_for_character(
    pool: &SqlitePool,
    character_id: &str,
) -> anyhow::Result<Vec<Chat>> {
    validate_id(character_id, "Character")?;
    let records = sqlx::query_as::<_, ChatRecord>(
        "SELECT id, character_id, persona_id, title, created_at, updated_at FROM chats \
         WHERE character_id = ? ORDER BY updated_at DESC, rowid DESC",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await
    .context("could not load chats")?;
    Ok(records.into_iter().map(chat_from_record).collect())
}

pub async fn get_chat_for_character(
    pool: &SqlitePool,
    character_id: &str,
    chat_id: &str,
) -> anyhow::Result<Option<Chat>> {
    validate_id(character_id, "Character")?;
    validate_id(chat_id, "Chat")?;
    let record = sqlx::query_as::<_, ChatRecord>(
        "SELECT id, character_id, persona_id, title, created_at, updated_at FROM chats \
         WHERE id = ? AND character_id = ?",
    )
    .bind(chat_id)
    .bind(character_id)
    .fetch_optional(pool)
    .await
    .context("could not load chat")?;
    Ok(record.map(chat_from_record))
}

pub async fn create_chat(
    pool: &SqlitePool,
    character_id: &str,
    title: &str,
    first_message: Option<&str>,
    persona_id: Option<&str>,
) -> anyhow::Result<Option<Chat>> {
    validate_id(character_id, "Character")?;
    let mut transaction = pool
        .begin()
        .await
        .context("could not start chat creation")?;
    let character_exists = sqlx::query_scalar::<_, i64>("SELECT 1 FROM characters WHERE id = ?")
        .bind(character_id)
        .fetch_optional(&mut *transaction)
        .await
        .context("could not verify character")?
        .is_some();
    if !character_exists {
        return Ok(None);
    }

    let chat_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO chats (id, character_id, persona_id, title) VALUES (?, ?, ?, ?)")
        .bind(&chat_id)
        .bind(character_id)
        .bind(persona_id)
        .bind(title)
        .execute(&mut *transaction)
        .await
        .context("could not create chat")?;

    if let Some(first_message) = first_message.filter(|content| !content.trim().is_empty()) {
        insert_message(
            &mut transaction,
            &chat_id,
            MessageRole::Assistant,
            first_message,
            None,
        )
        .await?;
    }
    transaction
        .commit()
        .await
        .context("could not finish chat creation")?;

    get_chat_for_character(pool, character_id, &chat_id).await
}

pub async fn set_chat_persona(
    pool: &SqlitePool,
    character_id: &str,
    chat_id: &str,
    persona_id: Option<&str>,
) -> anyhow::Result<Option<Chat>> {
    validate_id(character_id, "Character")?;
    validate_id(chat_id, "Chat")?;
    if let Some(persona_id) = persona_id {
        validate_id(persona_id, "Persona")?;
    }
    let result = sqlx::query(
        "UPDATE chats SET persona_id = ?, updated_at = STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ? AND character_id = ?",
    )
    .bind(persona_id)
    .bind(chat_id)
    .bind(character_id)
    .execute(pool)
    .await
    .context("could not update chat persona")?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    get_chat_for_character(pool, character_id, chat_id).await
}

pub async fn rename_chat(
    pool: &SqlitePool,
    character_id: &str,
    chat_id: &str,
    title: &str,
) -> anyhow::Result<Option<Chat>> {
    validate_id(character_id, "Character")?;
    validate_id(chat_id, "Chat")?;
    let result = sqlx::query(
        "UPDATE chats SET title = ?, updated_at = STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ? AND character_id = ?",
    )
    .bind(title)
    .bind(chat_id)
    .bind(character_id)
    .execute(pool)
    .await
    .context("could not rename chat")?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    get_chat_for_character(pool, character_id, chat_id).await
}

pub async fn delete_chat(
    pool: &SqlitePool,
    character_id: &str,
    chat_id: &str,
) -> anyhow::Result<Option<Chat>> {
    let chat = get_chat_for_character(pool, character_id, chat_id).await?;
    if chat.is_none() {
        return Ok(None);
    }
    sqlx::query("DELETE FROM chats WHERE id = ? AND character_id = ?")
        .bind(chat_id)
        .bind(character_id)
        .execute(pool)
        .await
        .context("could not delete chat")?;
    Ok(chat)
}

pub async fn list_chat_messages(
    pool: &SqlitePool,
    chat_id: &str,
) -> anyhow::Result<Vec<PersistedMessage>> {
    validate_id(chat_id, "Chat")?;
    let records = sqlx::query_as::<_, MessageRecord>(
        "SELECT id, chat_id, role, content, created_at FROM messages \
         WHERE chat_id = ? ORDER BY created_at ASC, rowid ASC",
    )
    .bind(chat_id)
    .fetch_all(pool)
    .await
    .context("could not load chat messages")?;
    records.into_iter().map(message_from_record).collect()
}

pub async fn append_user_message(
    pool: &SqlitePool,
    chat_id: &str,
    content: &str,
) -> anyhow::Result<PersistedMessage> {
    append_message(pool, chat_id, MessageRole::User, content, None).await
}

pub async fn append_assistant_message(
    pool: &SqlitePool,
    chat_id: &str,
    content: &str,
    generation_id: Option<&str>,
) -> anyhow::Result<PersistedMessage> {
    append_message(
        pool,
        chat_id,
        MessageRole::Assistant,
        content,
        generation_id,
    )
    .await
}

async fn append_message(
    pool: &SqlitePool,
    chat_id: &str,
    role: MessageRole,
    content: &str,
    generation_id: Option<&str>,
) -> anyhow::Result<PersistedMessage> {
    validate_id(chat_id, "Chat")?;
    validate_persisted_message(role, content)?;
    if let Some(generation_id) = generation_id {
        validate_id(generation_id, "Generation")?;
        if let Some(existing) = message_by_generation_id(pool, generation_id).await? {
            if existing.chat_id != chat_id {
                bail!("Generation does not belong to this chat.");
            }
            return Ok(existing);
        }
    }

    let mut transaction = pool.begin().await.context("could not start message save")?;
    let message = insert_message(&mut transaction, chat_id, role, content, generation_id).await?;
    sqlx::query("UPDATE chats SET updated_at = STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(chat_id)
        .execute(&mut *transaction)
        .await
        .context("could not update chat timestamp")?;
    transaction
        .commit()
        .await
        .context("could not finish message save")?;
    Ok(message)
}

async fn insert_message(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    chat_id: &str,
    role: MessageRole,
    content: &str,
    generation_id: Option<&str>,
) -> anyhow::Result<PersistedMessage> {
    validate_persisted_message(role, content)?;
    let message = PersistedMessage {
        id: Uuid::new_v4().to_string(),
        chat_id: chat_id.to_owned(),
        role,
        content: content.to_owned(),
        created_at: String::new(),
    };
    let saved = sqlx::query(
        "INSERT OR IGNORE INTO messages (id, chat_id, role, content, generation_id) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&message.id)
    .bind(&message.chat_id)
    .bind(message.role.as_database_value())
    .bind(&message.content)
    .bind(generation_id)
    .execute(&mut **transaction)
    .await
    .context("could not save message")?;

    // A browser cancellation and the provider stream can finish at nearly the
    // same time. `generation_id` makes that final write idempotent instead of
    // creating duplicate assistant messages or exposing a persistence race.
    if saved.rows_affected() == 0 {
        let generation_id = generation_id.context("message ID conflict")?;
        let existing = sqlx::query_as::<_, MessageRecord>(
            "SELECT id, chat_id, role, content, created_at FROM messages WHERE generation_id = ?",
        )
        .bind(generation_id)
        .fetch_one(&mut **transaction)
        .await
        .context("could not reload existing generated message")?;
        let existing = message_from_record(existing)?;
        if existing.chat_id != chat_id {
            bail!("Generation does not belong to this chat.");
        }
        return Ok(existing);
    }

    let created_at =
        sqlx::query_scalar::<_, String>("SELECT created_at FROM messages WHERE id = ?")
            .bind(&message.id)
            .fetch_one(&mut **transaction)
            .await
            .context("could not reload saved message")?;
    Ok(PersistedMessage {
        created_at,
        ..message
    })
}

async fn message_by_generation_id(
    pool: &SqlitePool,
    generation_id: &str,
) -> anyhow::Result<Option<PersistedMessage>> {
    let record = sqlx::query_as::<_, MessageRecord>(
        "SELECT id, chat_id, role, content, created_at FROM messages WHERE generation_id = ?",
    )
    .bind(generation_id)
    .fetch_optional(pool)
    .await
    .context("could not look up generated message")?;
    record.map(message_from_record).transpose()
}

fn chat_from_record(record: ChatRecord) -> Chat {
    Chat {
        id: record.id,
        character_id: record.character_id,
        persona_id: record.persona_id,
        title: record.title,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

fn message_from_record(record: MessageRecord) -> anyhow::Result<PersistedMessage> {
    Ok(PersistedMessage {
        id: record.id,
        chat_id: record.chat_id,
        role: MessageRole::from_database_value(&record.role)?,
        content: record.content,
        created_at: record.created_at,
    })
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
        domain::{character::CharacterDraft, persona::PersonaDraft},
        storage::{create_character, create_persona, delete_character, delete_persona},
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

    async fn character(pool: &SqlitePool, name: &str) -> crate::domain::character::Character {
        create_character(
            pool,
            CharacterDraft {
                name: name.to_owned(),
                ..CharacterDraft::default()
            },
        )
        .await
        .expect("character should be created")
    }

    #[tokio::test]
    async fn chats_are_isolated_and_first_message_is_created_once() {
        let pool = test_pool().await;
        let nyx = character(&pool, "Nyx").await;
        let luna = character(&pool, "Luna").await;
        let chat_a = create_chat(&pool, &nyx.id, "Main Story", Some("Welcome."), None)
            .await
            .expect("chat A")
            .expect("character exists");
        let chat_b = create_chat(&pool, &nyx.id, "Testing", None, None)
            .await
            .expect("chat B")
            .expect("character exists");
        let luna_chat = create_chat(&pool, &luna.id, "Luna Story", None, None)
            .await
            .expect("luna chat")
            .expect("character exists");

        append_user_message(&pool, &chat_a.id, "Message for chat A")
            .await
            .expect("message A");
        append_user_message(&pool, &chat_b.id, "Message for chat B")
            .await
            .expect("message B");
        append_user_message(&pool, &luna_chat.id, "Message for Luna")
            .await
            .expect("message Luna");
        let generation_id = "a0000000-0000-4000-8000-000000000099";
        append_assistant_message(
            &pool,
            &chat_a.id,
            "Response for chat A",
            Some(generation_id),
        )
        .await
        .expect("assistant response");
        let repeated = append_assistant_message(
            &pool,
            &chat_a.id,
            "This duplicate must not be stored",
            Some(generation_id),
        )
        .await
        .expect("idempotent assistant response");
        assert_eq!(repeated.content, "Response for chat A");

        let messages_a = list_chat_messages(&pool, &chat_a.id)
            .await
            .expect("messages A");
        assert_eq!(messages_a.len(), 3);
        assert_eq!(messages_a[0].content, "Welcome.");
        assert_eq!(messages_a[1].content, "Message for chat A");
        assert_eq!(messages_a[2].content, "Response for chat A");
        assert!(!messages_a
            .iter()
            .any(|message| message.content.contains("chat B")));
        assert!(get_chat_for_character(&pool, &luna.id, &chat_a.id)
            .await
            .expect("ownership check")
            .is_none());

        let nyx_sidebar_chats = list_chats_for_character(&pool, &nyx.id)
            .await
            .expect("Nyx sidebar chats");
        let luna_sidebar_chats = list_chats_for_character(&pool, &luna.id)
            .await
            .expect("Luna sidebar chats");
        assert_eq!(nyx_sidebar_chats.len(), 2);
        assert!(nyx_sidebar_chats
            .iter()
            .all(|chat| chat.character_id == nyx.id));
        assert_eq!(luna_sidebar_chats.len(), 1);
        assert_eq!(luna_sidebar_chats[0].id, luna_chat.id);
    }

    #[tokio::test]
    async fn chat_and_character_deletion_cascade_messages() {
        let pool = test_pool().await;
        let nyx = character(&pool, "Nyx").await;
        let chat = create_chat(&pool, &nyx.id, "Main", None, None)
            .await
            .expect("chat")
            .expect("character exists");
        append_user_message(&pool, &chat.id, "Persist me")
            .await
            .expect("message");
        delete_chat(&pool, &nyx.id, &chat.id)
            .await
            .expect("delete chat");
        assert!(list_chat_messages(&pool, &chat.id)
            .await
            .expect("messages")
            .is_empty());

        let second = create_chat(&pool, &nyx.id, "Second", None, None)
            .await
            .expect("chat")
            .expect("character exists");
        append_user_message(&pool, &second.id, "Delete with character")
            .await
            .expect("message");
        delete_character(&pool, &nyx.id)
            .await
            .expect("delete character");
        assert!(list_chats_for_character(&pool, &nyx.id)
            .await
            .expect("chats")
            .is_empty());
        assert!(list_chat_messages(&pool, &second.id)
            .await
            .expect("messages")
            .is_empty());
    }

    #[tokio::test]
    async fn chats_keep_independent_persona_references_and_survive_persona_deletion() {
        let pool = test_pool().await;
        let nyx = character(&pool, "Nyx").await;
        let arturo = create_persona(
            &pool,
            PersonaDraft {
                name: "Arturo".to_owned(),
                ..PersonaDraft::default()
            },
        )
        .await
        .expect("Arturo should create");
        let aldric = create_persona(
            &pool,
            PersonaDraft {
                name: "Aldric".to_owned(),
                ..PersonaDraft::default()
            },
        )
        .await
        .expect("Aldric should create");
        let chat_a = create_chat(&pool, &nyx.id, "Arturo story", None, Some(&arturo.id))
            .await
            .expect("chat A")
            .expect("character exists");
        let chat_b = create_chat(&pool, &nyx.id, "Aldric story", None, Some(&aldric.id))
            .await
            .expect("chat B")
            .expect("character exists");

        assert_eq!(chat_a.persona_id.as_deref(), Some(arturo.id.as_str()));
        assert_eq!(chat_b.persona_id.as_deref(), Some(aldric.id.as_str()));
        set_chat_persona(&pool, &nyx.id, &chat_b.id, Some(&arturo.id))
            .await
            .expect("persona should change");
        assert_eq!(
            get_chat_for_character(&pool, &nyx.id, &chat_b.id)
                .await
                .expect("chat B")
                .expect("chat B exists")
                .persona_id
                .as_deref(),
            Some(arturo.id.as_str())
        );

        delete_persona(&pool, &arturo.id)
            .await
            .expect("persona should delete");
        assert!(get_chat_for_character(&pool, &nyx.id, &chat_a.id)
            .await
            .expect("chat A")
            .expect("chat A exists")
            .persona_id
            .is_none());
        assert!(get_chat_for_character(&pool, &nyx.id, &chat_b.id)
            .await
            .expect("chat B")
            .expect("chat B exists")
            .persona_id
            .is_none());
    }
}
