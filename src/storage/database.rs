use std::{path::PathBuf, str::FromStr};

use anyhow::Context;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn connect(location: &str) -> anyhow::Result<Self> {
        let options = sqlite_options(location)?;
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .context("could not open SQLite database")?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("could not apply database migrations")?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> SqlitePool {
        self.pool.clone()
    }
}

fn sqlite_options(location: &str) -> anyhow::Result<SqliteConnectOptions> {
    let options = if location.starts_with("sqlite:") {
        SqliteConnectOptions::from_str(location).context("invalid SQLite connection URL")?
    } else {
        SqliteConnectOptions::new().filename(PathBuf::from(location))
    };

    Ok(options
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal))
}
