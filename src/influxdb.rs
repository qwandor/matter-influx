use crate::{
    config::Config,
    matter::{CHANGING_ATTRIBUTES, ClusterValue, ClusterValueDetails, UNCHANGING_ATTRIBUTES},
};
use eyre::{Report, WrapErr};
use influx_db_client::{Client, Point, Precision, Value};
use log::{debug, info};
use matter_controller::{
    AttributePath, MatterController, NodeInfo, Subscription, SubscriptionEvent,
};
use std::{collections::HashMap, time::SystemTime};

const INFLUXDB_PRECISION: Option<Precision> = Some(Precision::Seconds);

/// Loops forever trying to read values from Matter nodes and send them to InfluxDB.
pub async fn poll_values(
    influxdb_client: Client,
    matter_controller: MatterController,
    config: Config,
) -> Result<(), Report> {
    // TODO: Repeat this at some point in case the nodes change.
    for node_info in matter_controller.nodes().await? {
        info!(
            "Getting info for {}: {}",
            node_info.node_id,
            node_info.label.as_deref().unwrap_or("(No name)")
        );
        let node = matter_controller.node(node_info.node_id);
        let unchanging_values = node
            .read(UNCHANGING_ATTRIBUTES)
            .await?
            .into_iter()
            .collect::<HashMap<_, _>>();
        let subscription = node
            .subscribe(
                &CHANGING_ATTRIBUTES,
                &[],
                config.minimum_interval_seconds,
                config.maximum_interval_seconds,
            )
            .await?;
        info!("Subscribed to node {}", node_info.node_id);
        // TODO: Spawn a task for this, and keep track of it somehow.
        handle_subscription(subscription, node_info, unchanging_values, &influxdb_client).await?;
    }
    Ok(())
}

async fn handle_subscription(
    mut subscription: Subscription,
    node_info: NodeInfo,
    unchanging_values: HashMap<AttributePath, matter_controller::Value>,
    influxdb_client: &Client,
) -> Result<(), Report> {
    while let Some(event) = subscription.next().await {
        debug!("event: {event:?}");
        if let SubscriptionEvent::Report(report) = event
            && let Some(details) = ClusterValueDetails::for_attribute_value(
                report.path,
                &report.value,
                &unchanging_values,
            )
        {
            info!("details: {details}");
            influxdb_client
                .write_point(
                    make_point(&node_info, SystemTime::now(), &details),
                    INFLUXDB_PRECISION,
                    None,
                )
                .await
                .wrap_err("Failed to send property value update to InfluxDB")?;
        }
    }

    Ok(())
}

/// Constructs an InfluxDB point for an attribute value.
fn make_point(
    node_info: &NodeInfo,
    timestamp: SystemTime,
    value_details: &ClusterValueDetails,
) -> Point<'static> {
    let mut point = Point::new(value_details.value.datatype_str())
        .add_timestamp(
            timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        )
        .add_field("value", Value::from(value_details.value))
        .add_tag("device_id", node_info.node_id.to_string())
        .add_tag("endpoint_id", value_details.path.endpoint.to_string())
        .add_tag("cluster_id", format!("{:04x}", value_details.path.cluster))
        .add_tag(
            "attribute_id",
            format!("{:04x}", value_details.path.attribute),
        )
        .add_tag("property_name", value_details.name);
    if let Some(label) = &node_info.label {
        point = point.add_tag("device_name", label.to_owned());
    }
    if let Some(unit) = value_details.unit {
        point = point.add_tag("unit", unit);
    }
    if let ClusterValue::Boolean(value) = value_details.value {
        point = point.add_field("value_int", if value { 1 } else { 0 });
    }
    point
}
