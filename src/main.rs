mod config;
mod controllers;
mod errors;
mod influxdb;
mod matter;

use crate::{
    config::Config,
    controllers::{commission, index},
    influxdb::poll_values,
};
use axum::{
    Router,
    routing::{get, post},
};
use eyre::{Context, Report};
use futures::TryFutureExt;
use influx_db_client::Client;
use log::info;
use matter_controller::{AttestationTrust, FabricConfig, FileStore, MatterController, MatterTime};
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::{net::TcpListener, try_join};
use tracing_subscriber::EnvFilter;

const RCAC_ID: u64 = 42;
const CONTROLLER_NODE_ID: u64 = 1;

#[tokio::main]
async fn main() -> Result<(), Report> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::from_file()?;
    let influxdb_client = if let Some(influxdb_config) = &config.influxdb {
        Some(influxdb_config.make_client()?)
    } else {
        None
    };

    let matter_controller =
        MatterController::builder(Arc::new(FileStore::new(&config.matter_data_path)))
            .attestation_trust(
                AttestationTrust::from_dirs(&config.paa_dir, &config.cd_dir).wrap_err_with(
                    || {
                        format!(
                            "Reading certificates from {:?} and {:?}",
                            config.paa_dir, config.cd_dir
                        )
                    },
                )?,
            )
            .build()
            .await?;
    if matter_controller.fabrics().await?.is_empty() {
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

    let poll_values_handle =
        maybe_poll_values(influxdb_client, matter_controller.clone(), config.clone());

    let state = AppState { matter_controller };

    let app = Router::new()
        .route("/", get(index::index))
        .route("/commission", get(commission::commission))
        .route("/commission", post(commission::submit))
        .with_state(Arc::new(state));
    let listener = TcpListener::bind(&config.webserver_address).await?;
    info!("Listening on {}", config.webserver_address);
    let axum_handle = axum::serve(listener, app);

    let _: ((), ()) = try_join!(poll_values_handle, axum_handle.into_future().err_into())?;
    Ok(())
}

struct AppState {
    matter_controller: MatterController,
}

/// If an InfluxDB client is provided then subscribes to relevant attributes on all nodes and sends their values to InfluxDB.
async fn maybe_poll_values(
    influxdb_client: Option<Client>,
    matter_controller: MatterController,
    config: Config,
) -> Result<(), Report> {
    if let Some(influxdb_client) = influxdb_client {
        poll_values(influxdb_client, matter_controller.clone(), config.clone()).await?;
    } else {
        info!("No InfluxDB configured, not polling values.");
    }
    Ok(())
}
