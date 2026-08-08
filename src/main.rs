mod config;
mod controllers;
mod errors;

use crate::{config::Config, controllers::index};
use axum::{Router, routing::get};
use eyre::Report;
use log::info;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Report> {
    pretty_env_logger::init();

    let config = Config::from_file()?;

    let app = Router::new().route("/", get(index::index));
    let listener = TcpListener::bind(&config.webserver_address).await?;
    info!("Listening on {}", config.webserver_address);
    axum::serve(listener, app).await?;

    Ok(())
}
