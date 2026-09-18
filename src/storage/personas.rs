use anyhow::{bail, Context};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::{
    domain::persona::{Persona, PersonaDraft},
    storage::avatar_url,
};

#[derive(FromRow)]
struct PersonaRecord {
    id: String,
    name: String,
    avatar_path: Option<String>,
    description: String,
    created_at: String,
    updated_at: String,
}

pub async fn list_personas(pool: &SqlitePool) -> anyhow::Result<Vec<Persona>> {
    let records = sqlx::query_as::<_, PersonaRecord>(
        "SELECT id, name, avatar_path, description, created_at, updated_at \
         FROM personas ORDER BY updated_at DESC, name COLLATE NOCASE ASC",
    )
    .fetch_all(pool)
    .await
    .context("could not load personas")?;
    Ok(records.into_iter().map(persona_from_record).collect())
}

pub async fn get_persona(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Persona>> {
    validate_persona_id(id)?;
    let record = sqlx::query_as::<_, PersonaRecord>(
        "SELECT id, name, avatar_path, description, created_at, updated_at \
         FROM personas WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("could not load persona")?;
    Ok(record.map(persona_from_record))
}

pub async fn create_persona(pool: &SqlitePool, draft: PersonaDraft) -> anyhow::Result<Persona> {
    let persona = Persona {
        id: Uuid::new_v4().to_string(),
        name: draft.name,
        avatar_path: draft.avatar_path,
        avatar_url: None,
        description: draft.description,
        created_at: String::new(),
        updated_at: String::new(),
    };
    sqlx::query("INSERT INTO personas (id, name, avatar_path, description) VALUES (?, ?, ?, ?)")
        .bind(&persona.id)
        .bind(&persona.name)
        .bind(&persona.avatar_path)
        .bind(&persona.description)
        .execute(pool)
        .await
        .context("could not create persona")?;
    get_persona(pool, &persona.id)
        .await?
        .context("could not reload created persona")
}

pub async fn update_persona(
    pool: &SqlitePool,
    id: &str,
    draft: PersonaDraft,
) -> anyhow::Result<Option<Persona>> {
    validate_persona_id(id)?;
    let result = sqlx::query(
        "UPDATE personas SET name = ?, avatar_path = ?, description = ?, \
         updated_at = STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(&draft.name)
    .bind(&draft.avatar_path)
    .bind(&draft.description)
    .bind(id)
    .execute(pool)
    .await
    .context("could not update persona")?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    get_persona(pool, id).await
}

pub async fn duplicate_persona(
    pool: &SqlitePool,
    id: &str,
    copied_avatar_path: Option<String>,
) -> anyhow::Result<Option<Persona>> {
    let source = get_persona(pool, id).await?;
    let Some(source) = source else {
        return Ok(None);
    };
    let name = duplicate_name(&source.name);
    create_persona(
        pool,
        PersonaDraft {
            name,
            avatar_path: copied_avatar_path,
            description: source.description,
        },
    )
    .await
    .map(Some)
}

pub async fn delete_persona(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<Persona>> {
    let persona = get_persona(pool, id).await?;
    let Some(persona) = persona else {
        return Ok(None);
    };
    sqlx::query("DELETE FROM personas WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("could not delete persona")?;
    Ok(Some(persona))
}

pub async fn count_chats_for_persona(pool: &SqlitePool, id: &str) -> anyhow::Result<i64> {
    validate_persona_id(id)?;
    sqlx::query_scalar("SELECT COUNT(*) FROM chats WHERE persona_id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .context("could not count persona chats")
}

fn persona_from_record(record: PersonaRecord) -> Persona {
    Persona {
        avatar_url: record.avatar_path.as_deref().and_then(avatar_url),
        id: record.id,
        name: record.name,
        avatar_path: record.avatar_path,
        description: record.description,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

fn validate_persona_id(id: &str) -> anyhow::Result<()> {
    if Uuid::parse_str(id).is_err() {
        bail!("Persona ID is invalid.");
    }
    Ok(())
}

fn duplicate_name(name: &str) -> String {
    let suffix = " Copy";
    let prefix: String = name
        .chars()
        .take(120usize.saturating_sub(suffix.chars().count()))
        .collect();
    format!("{prefix}{suffix}")
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
            .expect("migrations should apply");
        pool
    }

    #[tokio::test]
    async fn creates_updates_duplicates_and_deletes_personas() {
        let pool = test_pool().await;
        let persona = create_persona(
            &pool,
            PersonaDraft {
                name: "Arturo".to_owned(),
                description: "A thoughtful traveler".to_owned(),
                ..PersonaDraft::default()
            },
        )
        .await
        .expect("persona should create");
        let updated = update_persona(
            &pool,
            &persona.id,
            PersonaDraft {
                name: "Arturo Vale".to_owned(),
                description: "An observant traveler".to_owned(),
                ..PersonaDraft::default()
            },
        )
        .await
        .expect("persona should update")
        .expect("persona should exist");
        let duplicate = duplicate_persona(&pool, &updated.id, None)
            .await
            .expect("persona should duplicate")
            .expect("persona should exist");

        assert_ne!(duplicate.id, updated.id);
        assert_eq!(duplicate.name, "Arturo Vale Copy");
        assert_eq!(list_personas(&pool).await.expect("list personas").len(), 2);
        assert!(delete_persona(&pool, &updated.id)
            .await
            .expect("persona should delete")
            .is_some());
        assert!(get_persona(&pool, &updated.id)
            .await
            .expect("load persona")
            .is_none());
    }
}
