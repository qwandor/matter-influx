use crate::{AppState, errors::AppError};
use askama::Template;
use axum::{extract::State, response::Html};
use futures::future::join_all;
use matc::{
    clusters::{
        codec::{
            concentration_measurement, on_off, temperature_measurement, water_content_measurement,
        },
        defs::{
            CLUSTER_ID_CARBON_DIOXIDE_CONCENTRATION_MEASUREMENT,
            CLUSTER_ID_PM2_5_CONCENTRATION_MEASUREMENT,
            CLUSTER_RADON_CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREDVALUE,
            CLUSTER_RADON_CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREMENTUNIT,
        },
    },
    controller::Connection,
    devman::{Device, DeviceManager},
    tlv::TlvItemValue,
};
use std::sync::Arc;

pub async fn index(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let devices = state.device_manager.list_devices()?;

    let devices = join_all(
        devices
            .into_iter()
            .map(|device| DeviceInfo::for_device(device, &state.device_manager)),
    )
    .await;

    let template = IndexTemplate { devices };
    Ok(Html(template.render()?))
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    devices: Vec<DeviceInfo>,
}

#[derive(Clone, Debug)]
struct DeviceInfo {
    device: Device,
    error: Option<String>,
    info: Vec<String>,
}

impl DeviceInfo {
    async fn for_device(device: Device, device_manager: &DeviceManager) -> Self {
        match get_device_info(device.node_id, device_manager).await {
            Ok(info) => Self {
                device,
                error: None,
                info,
            },
            Err(e) => Self {
                device,
                error: Some(e.to_string()),
                info: Vec::new(),
            },
        }
    }
}

async fn get_device_info(
    node_id: u64,
    device_manager: &DeviceManager,
) -> Result<Vec<String>, anyhow::Error> {
    let connection = device_manager.connect(node_id).await?;
    let mut info = Vec::new();
    let on = on_off::read_on_off(&connection, 1).await?;
    info.push(if on { "On" } else { "Off" }.to_owned());
    if let Some(temperature) = temperature_measurement::read_measured_value(&connection, 1).await? {
        info.push(format!(
            "Temperature: {}.{} °C",
            temperature / 100,
            temperature % 100
        ));
    }
    if let Some(humidity) = water_content_measurement::read_measured_value(&connection, 1).await? {
        info.push(format!("Humidity: {}.{} %", humidity / 100, humidity % 100));
    }
    if let Some(value) = read_attribute_value(
        &connection,
        1,
        CLUSTER_ID_PM2_5_CONCENTRATION_MEASUREMENT,
        CLUSTER_RADON_CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREDVALUE,
        concentration_measurement::decode_measured_value,
    )
    .await?
        && let unit = read_attribute_value(
            &connection,
            1,
            CLUSTER_ID_PM2_5_CONCENTRATION_MEASUREMENT,
            CLUSTER_RADON_CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREMENTUNIT,
            concentration_measurement::decode_measurement_unit,
        )
        .await?
    {
        info.push(format!("PM2.5: {} {:?}", value, unit));
    }
    if let Some(value) = read_attribute_value(
        &connection,
        1,
        CLUSTER_ID_CARBON_DIOXIDE_CONCENTRATION_MEASUREMENT,
        CLUSTER_RADON_CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREDVALUE,
        concentration_measurement::decode_measured_value,
    )
    .await?
        && let unit = read_attribute_value(
            &connection,
            1,
            CLUSTER_ID_CARBON_DIOXIDE_CONCENTRATION_MEASUREMENT,
            CLUSTER_RADON_CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREMENTUNIT,
            concentration_measurement::decode_measurement_unit,
        )
        .await?
    {
        info.push(format!("CO2: {} {:?}", value, unit));
    }
    Ok(info)
}

async fn read_attribute_value<T>(
    connection: &Connection,
    endpoint: u16,
    cluster: u32,
    attribute: u32,
    decode: fn(&TlvItemValue) -> Result<T, anyhow::Error>,
) -> Result<T, anyhow::Error> {
    let value = connection
        .read_request2(endpoint, cluster, attribute)
        .await?;
    decode(&value)
}
