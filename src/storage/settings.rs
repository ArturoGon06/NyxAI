use anyhow::Context;
use sqlx::SqlitePool;

use crate::domain::settings::{AppSettings, AppearanceSettings};

const SETTINGS_KEY: &str = "application_settings";

pub async fn load_settings(pool: &SqlitePool) -> anyhow::Result<AppSettings> {
    let stored = sqlx::query_scalar::<_, String>("SELECT value FROM app_settings WHERE key = ?")
        .bind(SETTINGS_KEY)
        .fetch_optional(pool)
        .await
        .context("could not load application settings")?;

    let mut settings: AppSettings = match stored {
        Some(json) => {
            let mut value: serde_json::Value =
                serde_json::from_str(&json).context("stored application settings are invalid")?;
            if let Some(fields) = value.as_object_mut() {
                let appearance = AppearanceSettings::from_persisted_value(fields.get("appearance"));
                fields.insert(
                    "appearance".to_owned(),
                    serde_json::to_value(appearance)
                        .context("could not normalize stored appearance settings")?,
                );
            }
            serde_json::from_value(value).context("stored application settings are invalid")?
        }
        None => AppSettings::default(),
    };
    settings.appearance.normalize_or_default();
    Ok(settings)
}

pub async fn save_settings(pool: &SqlitePool, settings: &AppSettings) -> anyhow::Result<()> {
    let serialized =
        serde_json::to_string(settings).context("could not encode application settings")?;

    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(SETTINGS_KEY)
    .bind(serialized)
    .execute(pool)
    .await
    .context("could not save application settings")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    #[tokio::test]
    async fn persists_extended_appearance_settings() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test database should connect");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations should apply");

        let mut settings = AppSettings::default();
        settings.appearance.secondary_background = "#161220".to_owned();
        settings.appearance.muted_text = "#B0A8B8".to_owned();
        settings.persona.default_persona_id =
            Some("a0000000-0000-4000-8000-000000000001".to_owned());
        save_settings(&pool, &settings)
            .await
            .expect("settings should save");

        assert_eq!(
            load_settings(&pool).await.expect("settings should load"),
            settings
        );
    }

    #[tokio::test]
    async fn loading_settings_defaults_only_malformed_appearance_fields() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("test database should connect");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations should apply");

        sqlx::query("INSERT INTO app_settings (key, value) VALUES (?, ?)")
            .bind(SETTINGS_KEY)
            .bind(r##"{"appearance":{"app_background":42,"accent_color":"#b78d24"}}"##)
            .execute(&pool)
            .await
            .expect("damaged test settings should save");

        let settings = load_settings(&pool)
            .await
            .expect("damaged appearance data should be recoverable");

        assert_eq!(settings.appearance.app_background, "#08080B");
        assert_eq!(settings.appearance.accent_color, "#B78D24");
        assert_eq!(settings.appearance.muted_text, "#9A94A3");
    }
}
