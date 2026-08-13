use crate::{AppState, errors::AppError, matter::read_all_known_clusters};
use askama::Template;
use axum::{extract::State, response::Html};
use futures::future::join_all;
use matter_controller::{MatterController, NodeInfo};
use std::sync::Arc;

pub async fn index(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let nodes = state.matter_controller.nodes().await?;

    let nodes = join_all(
        nodes
            .into_iter()
            .map(|node| DeviceInfo::for_node(node, &state.matter_controller)),
    )
    .await;

    let template = IndexTemplate { nodes };
    Ok(Html(template.render()?))
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    nodes: Vec<DeviceInfo>,
}

#[derive(Clone, Debug)]
struct DeviceInfo {
    node: NodeInfo,
    error: Option<String>,
    info: Vec<String>,
}

impl DeviceInfo {
    async fn for_node(node: NodeInfo, matter_controller: &MatterController) -> Self {
        match get_device_info(node.node_id, matter_controller).await {
            Ok(info) => Self {
                node,
                error: None,
                info,
            },
            Err(e) => Self {
                node,
                error: Some(e.to_string()),
                info: Vec::new(),
            },
        }
    }
}

async fn get_device_info(
    node_id: u64,
    matter_controller: &MatterController,
) -> Result<Vec<String>, anyhow::Error> {
    let node = matter_controller.node(node_id);
    Ok(read_all_known_clusters(&node, 1)
        .await?
        .into_iter()
        .map(|cluster_value| cluster_value.to_string())
        .collect())
}
