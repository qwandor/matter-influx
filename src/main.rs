mod config;
mod controllers;
mod errors;

use crate::{
    config::Config,
    controllers::{commission, index},
};
use axum::{
    Router,
    routing::{get, post},
};
use log::info;
use matc::devman::{DeviceManager, ManagerConfig};
use std::{path::Path, sync::Arc};
use tokio::net::TcpListener;

const CONTROLLER_ID: u64 = 1;
const MINIMUM_NODE_ID: u64 = 2;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    pretty_env_logger::init();

    let config = Config::from_file()?;

    let matter_config = ManagerConfig {
        fabric_id: config.matter_fabric_id,
        controller_id: CONTROLLER_ID,
        local_address: config.matter_controller_address.to_string(),
    };
    let device_manager = if Path::new(&config.matter_data_path).exists() {
        DeviceManager::load(&config.matter_data_path).await
    } else {
        DeviceManager::create(&config.matter_data_path, matter_config).await
    }?;

    let state = AppState { device_manager };

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
    device_manager: DeviceManager,
}

impl AppState {
    fn next_node_id(&self) -> Result<u64, anyhow::Error> {
        Ok(self
            .device_manager
            .list_devices()?
            .into_iter()
            .map(|device| device.node_id)
            .max()
            .map(|max_node_id| max_node_id + 1)
            .unwrap_or(MINIMUM_NODE_ID))
    }
}
