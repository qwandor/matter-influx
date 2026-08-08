use crate::{AppState, errors::AppError};
use askama::Template;
use axum::{extract::State, response::Html};
use matc::devman::Device;
use std::sync::Arc;

pub async fn index(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let devices = state.device_manager.list_devices()?;

    let template = IndexTemplate { devices };
    Ok(Html(template.render()?))
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    devices: Vec<Device>,
}
