use anyhow::{bail, Context};
use sqlx::{FromRow, Sqlite, SqlitePool, Transaction};
use uuid::Uuid;

use crate::{
    domain::character::{Character, CharacterDraft, CharacterGreeting},
    storage::avatar_url,
};

#[derive(FromRow)]
struct CharacterRecord {
    id: String,
    name: String,
    avatar_path: Option<String>,
    description: String,
    personality: String,
    scenario: String,
    first_message: String,
    example_dialogue: String,
    system_prompt: String,
    creator_notes: String,
    tags_json: String,
    created_at: String,
    updated_at: String,
}

#[derive(FromRow)]
struct CharacterGreetingRecord {
    id: String,
    content: String,
    position: i64,
}

pub async fn list_characters(pool: &SqlitePool) -> anyhow::Result<Vec<Character>> {
    let records = sqlx::query_as::<_, CharacterRecord>(
        "SELECT id, name, avatar_path, description, personality, scenario, first_message, \
         example_dialogue, system_prompt, creator_notes, tags_json, created_at, updated_at \
         FROM characters ORDER BY updated_at DESC, name COLLATE NOCASE ASC",
    )
    .fetch_all(pool)
    .await
    .context("could not load characters")?;

    let mut characters = Vec::with_capacity(records.len());
    for record in records {
        characters.push(character_from_record(pool, record).await?);
    }
    Ok(characters)
}

pub async fn get_character(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Character>> {
    validate_character_id(id)?;
    let record = sqlx::query_as::<_, CharacterRecord>(
        "SELECT id, name, avatar_path, description, personality, scenario, first_message, \
         example_dialogue, system_prompt, creator_notes, tags_json, created_at, updated_at \
         FROM characters WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("could not load character")?;

    match record {
        Some(record) => character_from_record(pool, record).await.map(Some),
        None => Ok(None),
    }
}

pub async fn create_character(
    pool: &SqlitePool,
    draft: CharacterDraft,
) -> anyhow::Result<Character> {
    let draft = draft.normalize_and_validate()?;
    let id = Uuid::new_v4().to_string();
    let mut transaction = pool
        .begin()
        .await
        .context("could not start character save")?;
    write_character(&mut transaction, &id, &draft, false).await?;
    replace_greetings(&mut transaction, &id, &draft.alternate_greetings).await?;
    transaction
        .commit()
        .await
        .context("could not finish character save")?;
    get_character(pool, &id)
        .await?
        .context("created character could not be reloaded")
}

pub async fn update_character(
    pool: &SqlitePool,
    id: &str,
    draft: CharacterDraft,
) -> anyhow::Result<Option<Character>> {
    validate_character_id(id)?;
    let draft = draft.normalize_and_validate()?;
    let mut transaction = pool
        .begin()
        .await
        .context("could not start character update")?;
    let result = write_character(&mut transaction, id, &draft, true).await?;
    if result.rows_affected() == 0 {
        transaction
            .rollback()
            .await
            .context("could not cancel character update")?;
        return Ok(None);
    }
    replace_greetings(&mut transaction, id, &draft.alternate_greetings).await?;
    transaction
        .commit()
        .await
        .context("could not finish character update")?;
    get_character(pool, id).await
}

pub async fn delete_character(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Character>> {
    let character = get_character(pool, id).await?;
    if character.is_none() {
        return Ok(None);
    }

    sqlx::query("DELETE FROM characters WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("could not delete character")?;
    Ok(character)
}

pub async fn duplicate_character(
    pool: &SqlitePool,
    source_id: &str,
    copied_avatar_path: Option<String>,
) -> anyhow::Result<Option<Character>> {
    let Some(source) = get_character(pool, source_id).await? else {
        return Ok(None);
    };

    let duplicate = CharacterDraft {
        name: format!("{} Copy", source.name),
        avatar_path: copied_avatar_path,
        description: source.description,
        personality: source.personality,
        scenario: source.scenario,
        first_message: source.first_message,
        example_dialogue: source.example_dialogue,
        system_prompt: source.system_prompt,
        creator_notes: source.creator_notes,
        tags: source.tags,
        alternate_greetings: source
            .alternate_greetings
            .into_iter()
            .map(|greeting| greeting.content)
            .collect(),
    };
    create_character(pool, duplicate).await.map(Some)
}

async fn write_character(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    draft: &CharacterDraft,
    update: bool,
) -> anyhow::Result<sqlx::sqlite::SqliteQueryResult> {
    let tags_json =
        serde_json::to_string(&draft.tags).context("could not encode character tags")?;
    if !update {
        return sqlx::query(
            "INSERT INTO characters (id, name, avatar_path, description, personality, scenario, \
             first_message, example_dialogue, system_prompt, creator_notes, tags_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&draft.name)
        .bind(&draft.avatar_path)
        .bind(&draft.description)
        .bind(&draft.personality)
        .bind(&draft.scenario)
        .bind(&draft.first_message)
        .bind(&draft.example_dialogue)
        .bind(&draft.system_prompt)
        .bind(&draft.creator_notes)
        .bind(tags_json)
        .execute(&mut **transaction)
        .await
        .context("could not create character");
    }

    sqlx::query(
        "UPDATE characters SET name = ?, avatar_path = ?, description = ?, personality = ?, \
         scenario = ?, first_message = ?, example_dialogue = ?, system_prompt = ?, creator_notes = ?, tags_json = ?, \
         updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(&draft.name)
    .bind(&draft.avatar_path)
    .bind(&draft.description)
    .bind(&draft.personality)
    .bind(&draft.scenario)
    .bind(&draft.first_message)
    .bind(&draft.example_dialogue)
    .bind(&draft.system_prompt)
    .bind(&draft.creator_notes)
    .bind(tags_json)
    .bind(id)
    .execute(&mut **transaction)
    .await
    .context("could not update character")
}

async fn replace_greetings(
    transaction: &mut Transaction<'_, Sqlite>,
    character_id: &str,
    greetings: &[String],
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM character_greetings WHERE character_id = ?")
        .bind(character_id)
        .execute(&mut **transaction)
        .await
        .context("could not replace alternate greetings")?;

    for (position, content) in greetings.iter().enumerate() {
        sqlx::query(
            "INSERT INTO character_greetings (id, character_id, position, content) VALUES (?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(character_id)
        .bind(position as i64)
        .bind(content)
        .execute(&mut **transaction)
        .await
        .context("could not save alternate greeting")?;
    }
    Ok(())
}

async fn character_from_record(
    pool: &SqlitePool,
    record: CharacterRecord,
) -> anyhow::Result<Character> {
    let tags =
        serde_json::from_str(&record.tags_json).context("stored character tags are invalid")?;
    let avatar_url = record.avatar_path.as_deref().and_then(avatar_url);
    let alternate_greetings = list_alternate_greetings(pool, &record.id).await?;
    Ok(Character {
        id: record.id,
        name: record.name,
        avatar_path: record.avatar_path,
        avatar_url,
        description: record.description,
        personality: record.personality,
        scenario: record.scenario,
        first_message: record.first_message,
        example_dialogue: record.example_dialogue,
        system_prompt: record.system_prompt,
        creator_notes: record.creator_notes,
        tags,
        alternate_greetings,
        created_at: record.created_at,
        updated_at: record.updated_at,
    })
}

pub async fn list_alternate_greetings(
    pool: &SqlitePool,
    character_id: &str,
) -> anyhow::Result<Vec<CharacterGreeting>> {
    validate_character_id(character_id)?;
    let records = sqlx::query_as::<_, CharacterGreetingRecord>(
        "SELECT id, content, position FROM character_greetings WHERE character_id = ? ORDER BY position ASC",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await
    .context("could not load alternate greetings")?;
    Ok(records
        .into_iter()
        .map(|record| CharacterGreeting {
            id: record.id,
            content: record.content,
            position: record.position,
        })
        .collect())
}

fn validate_character_id(id: &str) -> anyhow::Result<()> {
    if Uuid::parse_str(id).is_err() {
        bail!("Character ID is invalid.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test database should connect");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("character migration should apply");
        pool
    }

    fn draft(name: &str) -> CharacterDraft {
        CharacterDraft {
            name: name.to_owned(),
            description: "A midnight companion".to_owned(),
            tags: vec!["companion".to_owned()],
            ..CharacterDraft::default()
        }
    }

    #[tokio::test]
    async fn creates_updates_duplicates_and_deletes_characters() {
        let pool = test_pool().await;
        let created = create_character(&pool, draft("Nyx"))
            .await
            .expect("create should work");
        assert_eq!(list_characters(&pool).await.expect("list").len(), 1);

        let mut updated_draft = draft("Nyx Updated");
        updated_draft.first_message = "Welcome back.".to_owned();
        updated_draft.creator_notes = "Imported notes.".to_owned();
        updated_draft.alternate_greetings =
            vec!["A different opening.".to_owned(), "Again.".to_owned()];
        let updated = update_character(&pool, &created.id, updated_draft)
            .await
            .expect("update should work")
            .expect("character should exist");
        assert_eq!(updated.first_message, "Welcome back.");
        assert_eq!(updated.creator_notes, "Imported notes.");
        assert_eq!(
            updated
                .alternate_greetings
                .iter()
                .map(|greeting| greeting.content.as_str())
                .collect::<Vec<_>>(),
            vec!["A different opening.", "Again."]
        );

        let duplicate = duplicate_character(&pool, &created.id, None)
            .await
            .expect("duplicate should work")
            .expect("character should exist");
        assert_ne!(duplicate.id, created.id);
        assert_eq!(duplicate.name, "Nyx Updated Copy");
        assert_eq!(duplicate.alternate_greetings.len(), 2);

        let deleted = delete_character(&pool, &created.id)
            .await
            .expect("delete should work");
        assert_eq!(deleted.expect("deleted character").id, created.id);
        assert!(get_character(&pool, &created.id)
            .await
            .expect("get")
            .is_none());
        assert_eq!(list_characters(&pool).await.expect("list").len(), 1);
    }
}
