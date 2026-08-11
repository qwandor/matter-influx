use crate::{AppState, errors::AppError};
use askama::Template;
use axum::{extract::State, response::Html};
use futures::future::join_all;
use matter_clusters::r#gen::{on_off, relative_humidity_measurement, temperature_measurement};
use matter_controller::{MatterController, NodeInfo, ReadPath, Value};
use std::sync::Arc;

const CLUSTER_ID_PM2_5_CONCENTRATION_MEASUREMENT: u32 = 0x042a;
const CLUSTER_ID_CARBON_DIOXIDE_CONCENTRATION_MEASUREMENT: u32 = 0x040d;
const CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREDVALUE: u32 = 0x0000;
const CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREMENTUNIT: u32 = 0x0008;

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
    let mut info = Vec::new();
    let on = node
        .read(&[ReadPath::cluster(1, on_off::CLUSTER_ID)])
        .await?;
    info.push(format!("{on:?}"));
    if let &[(_, Value::Bool(on))] = node
        .read(&[ReadPath::concrete(
            1,
            on_off::CLUSTER_ID,
            on_off::attribute_id::ON_OFF,
        )])
        .await?
        .as_slice()
    {
        info.push(if on { "On" } else { "Off" }.to_owned());
    }

    if let &[(_, Value::Int(temperature))] = node
        .read(&[ReadPath::concrete(
            1,
            temperature_measurement::CLUSTER_ID,
            temperature_measurement::attribute_id::MEASURED_VALUE,
        )])
        .await?
        .as_slice()
    {
        info.push(format!(
            "Temperature: {}.{} °C",
            temperature / 100,
            temperature % 100
        ));
    }
    if let &[(_, Value::Uint(humidity))] = node
        .read(&[ReadPath::concrete(
            1,
            relative_humidity_measurement::CLUSTER_ID,
            relative_humidity_measurement::attribute_id::MEASURED_VALUE,
        )])
        .await?
        .as_slice()
    {
        info.push(format!("Humidity: {}.{} %", humidity / 100, humidity % 100));
    }
    if let [(_, Value::Float(value)), (_, Value::Bytes(unit))] = node
        .read(&[
            ReadPath::concrete(
                1,
                CLUSTER_ID_PM2_5_CONCENTRATION_MEASUREMENT,
                CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREDVALUE,
            ),
            ReadPath::concrete(
                1,
                CLUSTER_ID_PM2_5_CONCENTRATION_MEASUREMENT,
                CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREMENTUNIT,
            ),
        ])
        .await?
        .as_slice()
    {
        info.push(format!("PM2.5: {} {:?}", value, unit));
    }
    if let [(_, Value::Float(value)), (_, Value::Bytes(unit))] = node
        .read(&[
            ReadPath::concrete(
                1,
                CLUSTER_ID_CARBON_DIOXIDE_CONCENTRATION_MEASUREMENT,
                CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREDVALUE,
            ),
            ReadPath::concrete(
                1,
                CLUSTER_ID_CARBON_DIOXIDE_CONCENTRATION_MEASUREMENT,
                CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREMENTUNIT,
            ),
        ])
        .await?
        .as_slice()
    {
        info.push(format!("CO2: {} {:?}", value, unit));
    }
    Ok(info)
}
