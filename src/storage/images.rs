use anyhow::{bail, Context};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::domain::image::ChatImage;

use super::avatars::remove_avatar;

#[derive(FromRow)]
struct ChatImageRecord {
    id: String,
    chat_id: String,
    file_path: String,
    prompt: String,
    model: Option<String>,
    width: i64,
    height: i64,
    created_at: String,
}

pub async fn create_chat_image(
    pool: &SqlitePool,
    chat_id: &str,
    file_path: &str,
    prompt: &str,
    model: Option<&str>,
    width: u32,
    height: u32,
) -> anyhow::Result<ChatImage> {
    validate_uuid(chat_id, "Chat")?;
    ensure_reference(file_path)?;
    let image_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO chat_images (id, chat_id, file_path, prompt, model, width, height) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(&image_id).bind(chat_id).bind(file_path).bind(prompt).bind(model).bind(i64::from(width)).bind(i64::from(height))
        .execute(pool).await.context("could not save generated image metadata")?;
    get_chat_image(pool, chat_id, &image_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("saved chat image could not be loaded"))
}

pub async fn list_chat_images(pool: &SqlitePool, chat_id: &str) -> anyhow::Result<Vec<ChatImage>> {
    validate_uuid(chat_id, "Chat")?;
    let records = sqlx::query_as::<_, ChatImageRecord>("SELECT id, chat_id, file_path, prompt, model, width, height, created_at FROM chat_images WHERE chat_id = ? ORDER BY created_at ASC, rowid ASC")
        .bind(chat_id).fetch_all(pool).await.context("could not load generated chat images")?;
    records.into_iter().map(record_to_image).collect()
}

pub async fn get_chat_image(
    pool: &SqlitePool,
    chat_id: &str,
    image_id: &str,
) -> anyhow::Result<Option<ChatImage>> {
    validate_uuid(chat_id, "Chat")?;
    validate_uuid(image_id, "Image")?;
    let record = sqlx::query_as::<_, ChatImageRecord>("SELECT id, chat_id, file_path, prompt, model, width, height, created_at FROM chat_images WHERE chat_id = ? AND id = ?")
        .bind(chat_id).bind(image_id).fetch_optional(pool).await.context("could not load generated chat image")?;
    record.map(record_to_image).transpose()
}

pub async fn delete_chat_image(
    pool: &SqlitePool,
    chat_id: &str,
    image_id: &str,
) -> anyhow::Result<Option<String>> {
    let image = get_chat_image(pool, chat_id, image_id).await?;
    if let Some(image) = image {
        sqlx::query("DELETE FROM chat_images WHERE id = ? AND chat_id = ?")
            .bind(image_id)
            .bind(chat_id)
            .execute(pool)
            .await
            .context("could not delete generated image metadata")?;
        return Ok(Some(
            image.image_url.trim_start_matches("/images/").to_owned(),
        ));
    }
    Ok(None)
}

pub async fn remove_chat_image_file(
    directory: &std::path::Path,
    reference: &str,
) -> anyhow::Result<()> {
    // Generated references use the same strict UUID filename policy as avatars.
    remove_avatar(directory, reference)
        .await
        .map_err(anyhow::Error::new)
}

fn record_to_image(record: ChatImageRecord) -> anyhow::Result<ChatImage> {
    ensure_reference(&record.file_path)?;
    Ok(ChatImage {
        id: record.id,
        chat_id: record.chat_id,
        image_url: format!("/images/{}", record.file_path),
        prompt: record.prompt,
        model: record.model,
        width: u32::try_from(record.width).context("stored image width is invalid")?,
        height: u32::try_from(record.height).context("stored image height is invalid")?,
        created_at: record.created_at,
    })
}

fn validate_uuid(value: &str, name: &str) -> anyhow::Result<()> {
    if Uuid::parse_str(value).is_err() {
        bail!("{name} ID is invalid.");
    }
    Ok(())
}
fn ensure_reference(reference: &str) -> anyhow::Result<()> {
    let valid = reference.rsplit_once('.').is_some_and(|(id, extension)| {
        Uuid::parse_str(id).is_ok() && matches!(extension, "png" | "jpg" | "gif" | "webp")
    });
    if !valid {
        bail!("Generated image reference is invalid.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal_in_generated_image_references() {
        assert!(ensure_reference("../image.png").is_err());
        assert!(ensure_reference("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa.png").is_ok());
    }

    #[tokio::test]
    async fn persists_images_only_for_their_own_chat() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test database should connect");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations should apply");
        let character = crate::storage::create_character(
            &pool,
            crate::domain::character::CharacterDraft {
                name: "Nyx".to_owned(),
                ..Default::default()
            },
        )
        .await
        .expect("character should create");
        let first = crate::storage::create_chat(&pool, &character.id, "First", None, None)
            .await
            .expect("first chat should create")
            .expect("character should exist");
        let second = crate::storage::create_chat(&pool, &character.id, "Second", None, None)
            .await
            .expect("second chat should create")
            .expect("character should exist");
        let image = create_chat_image(
            &pool,
            &first.id,
            "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa.png",
            "moonlit library",
            None,
            512,
            512,
        )
        .await
        .expect("image should save");
        assert_eq!(list_chat_images(&pool, &first.id).await.unwrap().len(), 1);
        assert!(list_chat_images(&pool, &second.id)
            .await
            .unwrap()
            .is_empty());
        assert!(delete_chat_image(&pool, &second.id, &image.id)
            .await
            .unwrap()
            .is_none());
    }
}
