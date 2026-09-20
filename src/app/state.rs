use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use sqlx::SqlitePool;

use crate::providers::{ImageProviderService, ProviderService};

/// Small process-local gate for incompatible GPU-heavy work. It intentionally
/// exposes no provider details to route handlers.
#[derive(Default)]
pub struct AIResourceCoordinator {
    active: AtomicBool,
}

/// Releases a text or image generation slot even when a client disconnects
/// while a streaming request is still in flight.
pub struct AIActivityGuard {
    coordinator: Arc<AIResourceCoordinator>,
    image: bool,
}

impl Drop for AIActivityGuard {
    fn drop(&mut self) {
        if self.image {
            self.coordinator.finish_image();
        } else {
            self.coordinator.finish_text();
        }
    }
}

impl AIResourceCoordinator {
    pub fn try_begin_image(self: &Arc<Self>) -> Option<AIActivityGuard> {
        self.begin_image().then(|| AIActivityGuard {
            coordinator: Arc::clone(self),
            image: true,
        })
    }

    pub fn try_begin_text(self: &Arc<Self>) -> Option<AIActivityGuard> {
        self.begin_text().then(|| AIActivityGuard {
            coordinator: Arc::clone(self),
            image: false,
        })
    }
}
impl AIResourceCoordinator {
    pub fn begin_image(&self) -> bool {
        self.active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
    pub fn finish_image(&self) {
        self.active.store(false, Ordering::Release);
    }
    pub fn begin_text(&self) -> bool {
        self.active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
    pub fn finish_text(&self) {
        self.active.store(false, Ordering::Release);
    }
}

#[derive(Clone)]
pub struct AppState {
    pub database: SqlitePool,
    pub providers: ProviderService,
    pub image_providers: ImageProviderService,
    pub resources: Arc<AIResourceCoordinator>,
    /// This is the environment-provided fallback. A saved application setting
    /// takes precedence so a user can change servers from the NyxAI UI.
    pub default_ollama_base_url: String,
    pub default_openai_compatible_base_url: String,
    /// Persistent storage for user-provided character avatars.
    pub avatar_directory: PathBuf,
    pub default_a1111_base_url: String,
    pub image_directory: PathBuf,
}

impl AppState {
    // AppState construction happens once at the application boundary. Keeping
    // the independently configurable storage locations and backend defaults
    // explicit makes startup wiring auditable without adding a mutable config
    // bag to request state.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        database: SqlitePool,
        providers: ProviderService,
        image_providers: ImageProviderService,
        default_ollama_base_url: String,
        default_openai_compatible_base_url: String,
        avatar_directory: PathBuf,
        default_a1111_base_url: String,
        image_directory: PathBuf,
    ) -> Self {
        Self {
            database,
            providers,
            image_providers,
            resources: Arc::new(AIResourceCoordinator::default()),
            default_ollama_base_url,
            default_openai_compatible_base_url,
            avatar_directory,
            default_a1111_base_url,
            image_directory,
        }
    }
}
