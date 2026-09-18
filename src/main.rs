mod app;
mod domain;
mod importer;
mod providers;
mod storage;

use std::net::SocketAddr;

use anyhow::Context;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    app::{router, AppConfig, AppState},
    providers::{ImageProviderService, ProviderService},
    storage::Database,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nyxai=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_environment()?;
    let database = Database::connect(&config.database_location)
        .await
        .context("failed to initialize the NyxAI database")?;
    let providers = ProviderService::new().context("failed to initialize AI provider service")?;
    let image_providers =
        ImageProviderService::new().context("failed to initialize image provider service")?;
    let app = router(AppState::new(
        database.pool(),
        providers,
        image_providers,
        config.ollama_base_url.clone(),
        config.avatar_directory.clone(),
        config.a1111_base_url.clone(),
        config.image_directory.clone(),
    ));

    let address = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("could not bind NyxAI to {address}"))?;

    info!(
        address = %address,
        database = %config.database_location,
        avatars = %config.avatar_directory.display(),
        "NyxAI is ready"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("NyxAI server stopped unexpectedly")
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("shutdown signal received");
}
