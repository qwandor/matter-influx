mod config;
mod controllers;
mod errors;
mod matter;

use crate::{
    config::Config,
    controllers::{commission, index},
};
use axum::{
    Router,
    routing::{get, post},
};
use log::info;
use matter_controller::{AttestationTrust, FabricConfig, FileStore, MatterController, MatterTime};
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

const RCAC_ID: u64 = 42;
const CONTROLLER_NODE_ID: u64 = 1;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::from_file()?;

    let data_exists = config.matter_data_path.exists();
    let matter_controller =
        MatterController::builder(Arc::new(FileStore::new(&config.matter_data_path)))
            .attestation_trust(AttestationTrust::from_dirs(
                &config.paa_dir,
                &config.cd_dir,
            )?)
            .build()
            .await?;
    if !data_exists {
        matter_controller
            .create_fabric(FabricConfig::new(
                config.matter_fabric_id,
                RCAC_ID,
                CONTROLLER_NODE_ID,
                (
                    MatterTime::from_unix_secs(
                        (SystemTime::now() - Duration::from_hours(72))
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    ),
                    MatterTime::NO_EXPIRY,
                ),
            ))
            .await?;
    }

    let state = AppState { matter_controller };

    let app = Router::new()
        .route("/", get(index::index))
        .route("/commission", get(commission::commission))
        .route("/commission", post(commission::submit))
        .with_state(Arc::new(state));
    let listener = TcpListener::bind(&config.webserver_address).await?;
    info!("Listening on {}", config.webserver_address);
    axum::serve(listener, app).await?;

    Ok(())
}

struct AppState {
    matter_controller: MatterController,
}
