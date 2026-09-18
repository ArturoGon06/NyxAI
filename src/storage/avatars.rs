use std::{
    io,
    path::{Path, PathBuf},
};

use tokio::{fs, io::AsyncWriteExt};
use uuid::Uuid;

const MAX_AVATAR_BYTES: usize = 4 * 1024 * 1024;
const ALLOWED_EXTENSIONS: [&str; 4] = ["png", "jpg", "gif", "webp"];

#[derive(Debug)]
pub enum AvatarStorageError {
    InvalidImage,
    TooLarge,
    InvalidReference,
    Storage(anyhow::Error),
}

impl AvatarStorageError {
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::InvalidImage => "Choose a PNG, JPEG, GIF, or WebP image.",
            Self::TooLarge => "Choose an image smaller than 4 MB.",
            Self::InvalidReference => "That avatar reference is invalid.",
            Self::Storage(_) => "Avatar storage is unavailable. Please try again.",
        }
    }
}

impl std::fmt::Display for AvatarStorageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidImage => write!(formatter, "unsupported avatar image"),
            Self::TooLarge => write!(formatter, "avatar image exceeds size limit"),
            Self::InvalidReference => write!(formatter, "unsafe avatar reference"),
            Self::Storage(error) => write!(formatter, "avatar storage error: {error:#}"),
        }
    }
}

impl std::error::Error for AvatarStorageError {}

pub async fn store_avatar(
    avatar_directory: &Path,
    bytes: &[u8],
) -> Result<String, AvatarStorageError> {
    store_image(avatar_directory, bytes).await
}

/// Stores a validated image under a generated, application-controlled name.
/// Used by avatars and generated chat images; callers never choose the path.
pub async fn store_image(directory: &Path, bytes: &[u8]) -> Result<String, AvatarStorageError> {
    if bytes.len() > MAX_AVATAR_BYTES {
        return Err(AvatarStorageError::TooLarge);
    }
    let extension = detect_image_extension(bytes).ok_or(AvatarStorageError::InvalidImage)?;
    fs::create_dir_all(directory).await.map_err(storage_error)?;

    let reference = format!("{}.{}", Uuid::new_v4(), extension);
    let path = avatar_path(directory, &reference)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await
        .map_err(storage_error)?;
    file.write_all(bytes).await.map_err(storage_error)?;
    file.flush().await.map_err(storage_error)?;

    Ok(reference)
}

pub async fn ensure_avatar_exists(
    avatar_directory: &Path,
    reference: &str,
) -> Result<(), AvatarStorageError> {
    let path = avatar_path(avatar_directory, reference)?;
    match fs::metadata(path).await {
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(_) => Err(AvatarStorageError::InvalidReference),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err(AvatarStorageError::InvalidReference)
        }
        Err(error) => Err(storage_error(error)),
    }
}

pub async fn copy_avatar(
    avatar_directory: &Path,
    reference: &str,
) -> Result<String, AvatarStorageError> {
    let source = avatar_path(avatar_directory, reference)?;
    ensure_avatar_exists(avatar_directory, reference).await?;

    let extension = reference
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .ok_or(AvatarStorageError::InvalidReference)?;
    let copied_reference = format!("{}.{}", Uuid::new_v4(), extension);
    let destination = avatar_path(avatar_directory, &copied_reference)?;
    fs::copy(source, destination).await.map_err(storage_error)?;
    Ok(copied_reference)
}

pub async fn remove_avatar(
    avatar_directory: &Path,
    reference: &str,
) -> Result<(), AvatarStorageError> {
    let path = avatar_path(avatar_directory, reference)?;
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(storage_error(error)),
    }
}

pub fn is_safe_avatar_reference(reference: &str) -> bool {
    let Some((stem, extension)) = reference.rsplit_once('.') else {
        return false;
    };
    Uuid::parse_str(stem).is_ok()
        && ALLOWED_EXTENSIONS.contains(&extension)
        && Path::new(reference)
            .file_name()
            .is_some_and(|name| name == reference)
}

pub fn avatar_url(reference: &str) -> Option<String> {
    is_safe_avatar_reference(reference).then(|| format!("/avatars/{reference}"))
}

fn avatar_path(avatar_directory: &Path, reference: &str) -> Result<PathBuf, AvatarStorageError> {
    if !is_safe_avatar_reference(reference) {
        return Err(AvatarStorageError::InvalidReference);
    }
    Ok(avatar_directory.join(reference))
}

fn storage_error(error: io::Error) -> AvatarStorageError {
    AvatarStorageError::Storage(anyhow::Error::new(error))
}

fn detect_image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("jpg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_image_headers_without_trusting_file_names() {
        assert_eq!(
            detect_image_extension(b"\x89PNG\r\n\x1a\nrest"),
            Some("png")
        );
        assert_eq!(
            detect_image_extension(&[0xff, 0xd8, 0xff, 0xe0]),
            Some("jpg")
        );
        assert_eq!(detect_image_extension(b"not an image"), None);
    }

    #[test]
    fn only_accepts_application_controlled_avatar_references() {
        let reference = "1a4ac2e4-c24f-4d55-b052-a1e5b64f8057.png";
        assert!(is_safe_avatar_reference(reference));
        assert_eq!(
            avatar_url(reference).as_deref(),
            Some("/avatars/1a4ac2e4-c24f-4d55-b052-a1e5b64f8057.png")
        );
        assert!(!is_safe_avatar_reference("../avatar.png"));
        assert!(!is_safe_avatar_reference("avatar.svg"));
    }

    #[tokio::test]
    async fn stores_copies_and_removes_an_avatar_file() {
        let directory = std::env::temp_dir().join(format!("nyxai-avatar-test-{}", Uuid::new_v4()));
        let png = b"\x89PNG\r\n\x1a\nminimal test data";

        let original = store_avatar(&directory, png)
            .await
            .expect("avatar should be stored");
        ensure_avatar_exists(&directory, &original)
            .await
            .expect("stored avatar should exist");
        let copy = copy_avatar(&directory, &original)
            .await
            .expect("avatar should be copied");
        assert_ne!(original, copy);

        remove_avatar(&directory, &original)
            .await
            .expect("original should be removed");
        assert!(ensure_avatar_exists(&directory, &original).await.is_err());
        ensure_avatar_exists(&directory, &copy)
            .await
            .expect("copy should remain independent");
        remove_avatar(&directory, &copy)
            .await
            .expect("copy should be removed");
        fs::remove_dir_all(directory)
            .await
            .expect("test directory should be removed");
    }
}
