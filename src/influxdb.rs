use crate::{
    config::Config,
    matter::{CHANGING_ATTRIBUTES, ClusterValue, ClusterValueDetails, UNCHANGING_ATTRIBUTES},
};
use eyre::{Report, WrapErr};
use influx_db_client::{Client, Point, Precision, Value};
use log::{debug, info, warn};
use matter_controller::{
    AttributePath, MatterController, NodeInfo, Subscription, SubscriptionEvent,
};
use std::{
    collections::{BTreeMap, HashMap},
    time::{Duration, SystemTime},
};
use tokio::{spawn, task::JoinHandle, time::sleep};

/// Length of time to wait between polling the list of nodes again.
const SLEEP_BETWEEN_POLLING_NODES: Duration = Duration::from_secs(30);
const INFLUXDB_PRECISION: Option<Precision> = Some(Precision::Seconds);

/// Loops forever trying to read values from Matter nodes and send them to InfluxDB.
pub async fn poll_values(
    influxdb_client: Client,
    matter_controller: MatterController,
    config: Config,
) -> Result<(), Report> {
    let mut subscription_tasks = BTreeMap::<_, JoinHandle<()>>::new();

    loop {
        for node_info in matter_controller.nodes().await? {
            let node_id = node_info.node_id;
            // If there is already a task and it is still running then leave it alone.
            if let Some(handle) = subscription_tasks.get_mut(&node_id) {
                if handle.is_finished() {
                    if let Err(e) = handle.await {
                        warn!("Subscription task for node {node_id} finished with error: {e}");
                    }
                    subscription_tasks.remove(&node_id);
                } else {
                    continue;
                }
            }

            // Otherwise, try to subscribe and spawn a task
            info!(
                "Getting info for {node_id}: {}",
                node_info.label.as_deref().unwrap_or("(No name)")
            );
            let node = matter_controller.node(node_id);
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
            info!("Subscribed to node {}", node_id);
            // TODO: Spawn a task for this, and keep track of it somehow.
            let join_handle = spawn(handle_subscription_log_error(
                subscription,
                node_info,
                unchanging_values,
                influxdb_client.clone(),
            ));
            subscription_tasks.insert(node_id, join_handle);
        }
        sleep(SLEEP_BETWEEN_POLLING_NODES).await;
    }
}

async fn handle_subscription_log_error(
    subscription: Subscription,
    node_info: NodeInfo,
    unchanging_values: HashMap<AttributePath, matter_controller::Value>,
    influxdb_client: Client,
) {
    let node_id = node_info.node_id;
    if let Err(e) =
        handle_subscription(subscription, node_info, unchanging_values, influxdb_client).await
    {
        warn!("Error handling subscription to node {node_id}: {e}");
    } else {
        warn!("Subscription task for node {node_id} finished with no error");
    }
}

async fn handle_subscription(
    mut subscription: Subscription,
    node_info: NodeInfo,
    unchanging_values: HashMap<AttributePath, matter_controller::Value>,
    influxdb_client: Client,
) -> Result<(), Report> {
    while let Some(event) = subscription.next().await {
        debug!("event for node {}: {event:?}", node_info.node_id);
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
        // Grafana is unable to display booleans directly, so add an integer for convenience.
        // https://github.com/grafana/grafana/issues/8152
        // https://github.com/grafana/grafana/issues/24929
        point = point.add_field("value_int", if value { 1 } else { 0 });
    }
    point
}
