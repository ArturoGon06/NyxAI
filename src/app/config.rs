use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};

pub const DEFAULT_PORT: u16 = 8000;
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_DATABASE_LOCATION: &str = "./data/nyxai.db";
pub const DEFAULT_OLLAMA_BASE_URL: &str = "http://localhost:11434";
pub const DEFAULT_OPENAI_COMPATIBLE_BASE_URL: &str = "http://localhost:8080/v1";
pub const DEFAULT_A1111_BASE_URL: &str = "http://localhost:7860";

/// Runtime configuration intentionally stays small until a feature needs more.
pub struct AppConfig {
    pub port: u16,
    pub database_location: String,
    pub ollama_base_url: String,
    pub openai_compatible_base_url: String,
    /// Never log this value or return it through settings routes.
    pub openai_compatible_api_key: Option<String>,
    pub avatar_directory: PathBuf,
    pub a1111_base_url: String,
    pub image_directory: PathBuf,
}

impl AppConfig {
    pub fn from_environment() -> anyhow::Result<Self> {
        let port = env::var("NYXAI_PORT")
            .ok()
            .map(|value| {
                value
                    .parse::<u16>()
                    .with_context(|| "NYXAI_PORT must be a valid TCP port")
            })
            .transpose()?
            .unwrap_or(DEFAULT_PORT);

        if port == 0 {
            bail!("NYXAI_PORT must be between 1 and 65535");
        }

        let database_location =
            env::var("NYXAI_DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_LOCATION.to_owned());

        if database_location.trim().is_empty() {
            bail!("NYXAI_DATABASE_URL cannot be empty");
        }

        let ollama_base_url =
            env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| DEFAULT_OLLAMA_BASE_URL.to_owned());

        if ollama_base_url.trim().is_empty() {
            bail!("OLLAMA_BASE_URL cannot be empty");
        }

        let openai_compatible_base_url = env::var("OPENAI_COMPATIBLE_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_OPENAI_COMPATIBLE_BASE_URL.to_owned());
        if openai_compatible_base_url.trim().is_empty() {
            bail!("OPENAI_COMPATIBLE_BASE_URL cannot be empty");
        }
        let openai_compatible_api_key = env::var("OPENAI_COMPATIBLE_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty());

        let avatar_directory = env::var("NYXAI_ASSET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_avatar_directory(&database_location));

        if avatar_directory.as_os_str().is_empty() {
            bail!("NYXAI_ASSET_DIR cannot be empty");
        }

        let a1111_base_url =
            env::var("A1111_BASE_URL").unwrap_or_else(|_| DEFAULT_A1111_BASE_URL.to_owned());
        if a1111_base_url.trim().is_empty() {
            bail!("A1111_BASE_URL cannot be empty");
        }

        let image_directory = env::var("NYXAI_IMAGE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_image_directory(&database_location));
        if image_directory.as_os_str().is_empty() {
            bail!("NYXAI_IMAGE_DIR cannot be empty");
        }

        // A filesystem path is the documented default. SQLite URLs are also
        // accepted for deployments that already use SQLx-style configuration.
        if !database_location.starts_with("sqlite:") {
            let path = PathBuf::from(&database_location);
            if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("could not create database directory {}", parent.display())
                })?;
            }
        }

        std::fs::create_dir_all(&avatar_directory).with_context(|| {
            format!(
                "could not create avatar storage directory {}",
                avatar_directory.display()
            )
        })?;
        std::fs::create_dir_all(&image_directory).with_context(|| {
            format!(
                "could not create generated-image storage directory {}",
                image_directory.display()
            )
        })?;

        Ok(Self {
            port,
            database_location,
            ollama_base_url,
            openai_compatible_base_url,
            openai_compatible_api_key,
            avatar_directory,
            a1111_base_url,
            image_directory,
        })
    }
}

fn default_image_directory(database_location: &str) -> PathBuf {
    default_avatar_directory(database_location)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("images")
}

fn default_avatar_directory(database_location: &str) -> PathBuf {
    let database_path = database_location
        .strip_prefix("sqlite:")
        .filter(|path| !path.is_empty() && *path != ":memory:")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(database_location));

    database_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .join("avatars")
}

#[cfg(test)]
mod tests {
    use super::{
        APP_VERSION, DEFAULT_A1111_BASE_URL, DEFAULT_OLLAMA_BASE_URL,
        DEFAULT_OPENAI_COMPATIBLE_BASE_URL, DEFAULT_PORT,
    };

    #[test]
    fn default_port_matches_public_deployment_contract() {
        assert_eq!(DEFAULT_PORT, 8000);
    }

    #[test]
    fn default_ollama_address_is_a_local_development_server() {
        assert_eq!(DEFAULT_OLLAMA_BASE_URL, "http://localhost:11434");
    }

    #[test]
    fn default_openai_compatible_address_is_a_local_development_server() {
        assert_eq!(
            DEFAULT_OPENAI_COMPATIBLE_BASE_URL,
            "http://localhost:8080/v1"
        );
    }

    #[test]
    fn default_image_backend_address_is_a_local_development_server() {
        assert_eq!(DEFAULT_A1111_BASE_URL, "http://localhost:7860");
    }

    #[test]
    fn avatar_storage_defaults_next_to_a_file_database() {
        assert_eq!(
            super::default_avatar_directory("./data/nyxai.db"),
            std::path::PathBuf::from("./data/avatars")
        );
    }

    #[test]
    fn application_version_comes_from_the_package_manifest() {
        assert_eq!(APP_VERSION, env!("CARGO_PKG_VERSION"));
    }
}
