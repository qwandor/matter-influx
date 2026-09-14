use matter_clusters::r#gen::{
    carbon_dioxide_concentration_measurement, on_off, pm25_concentration_measurement, power_source,
    relative_humidity_measurement, temperature_measurement,
};
use matter_controller::{AttributePath, Node, ReadPath, Value};
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

/// Paths for attributes which we should read once, but aren't expected to change. These include
/// things like measurement units.
pub const UNCHANGING_ATTRIBUTES: &[ReadPath] = &[
    ReadPath::new(
        None,
        Some(pm25_concentration_measurement::CLUSTER_ID),
        Some(pm25_concentration_measurement::attribute_id::MEASUREMENT_UNIT),
    ),
    ReadPath::new(
        None,
        Some(carbon_dioxide_concentration_measurement::CLUSTER_ID),
        Some(carbon_dioxide_concentration_measurement::attribute_id::MEASUREMENT_UNIT),
    ),
];

/// Paths for attributes which we support and expect to change.
pub const CHANGING_ATTRIBUTES: &[ReadPath] = &[
    ReadPath::new(
        None,
        Some(on_off::CLUSTER_ID),
        Some(on_off::attribute_id::ON_OFF),
    ),
    ReadPath::new(
        None,
        Some(power_source::CLUSTER_ID),
        Some(power_source::attribute_id::BAT_PERCENT_REMAINING),
    ),
    ReadPath::new(
        None,
        Some(temperature_measurement::CLUSTER_ID),
        Some(temperature_measurement::attribute_id::MEASURED_VALUE),
    ),
    ReadPath::new(
        None,
        Some(relative_humidity_measurement::CLUSTER_ID),
        Some(relative_humidity_measurement::attribute_id::MEASURED_VALUE),
    ),
    ReadPath::new(
        None,
        Some(pm25_concentration_measurement::CLUSTER_ID),
        Some(pm25_concentration_measurement::attribute_id::MEASURED_VALUE),
    ),
    ReadPath::new(
        None,
        Some(carbon_dioxide_concentration_measurement::CLUSTER_ID),
        Some(carbon_dioxide_concentration_measurement::attribute_id::MEASURED_VALUE),
    ),
];

/// The value read from some cluster and parsed, ready to display.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct ClusterValueDetails {
    pub name: &'static str,
    pub value: ClusterValue,
    pub unit: Option<&'static str>,
}

impl ClusterValueDetails {
    pub fn for_attribute_value(
        path: &AttributePath,
        value: &Value,
        unchanging_values: &HashMap<AttributePath, Value>,
    ) -> Option<Self> {
        let endpoint = path.endpoint;
        match (path.cluster, path.attribute, value) {
            (on_off::CLUSTER_ID, on_off::attribute_id::ON_OFF, &Value::Bool(on)) => {
                Some(ClusterValueDetails {
                    name: "On",
                    value: ClusterValue::Boolean(on),
                    unit: None,
                })
            }
            (
                power_source::CLUSTER_ID,
                power_source::attribute_id::BAT_PERCENT_REMAINING,
                &Value::Uint(value),
            ) => Some(ClusterValueDetails {
                name: "Battery level",
                value: ClusterValue::Float(value as f32 / 2.0),
                unit: Some("%"),
            }),
            (
                temperature_measurement::CLUSTER_ID,
                temperature_measurement::attribute_id::MEASURED_VALUE,
                &Value::Int(value),
            ) => Some(ClusterValueDetails {
                name: "Temperature",
                value: ClusterValue::Float(value as f32 / 100.0),
                unit: Some("°C"),
            }),
            (
                relative_humidity_measurement::CLUSTER_ID,
                relative_humidity_measurement::attribute_id::MEASURED_VALUE,
                &Value::Uint(value),
            ) => Some(ClusterValueDetails {
                name: "Humidity",
                value: ClusterValue::Float(value as f32 / 100.0),
                unit: Some("%"),
            }),
            (
                pm25_concentration_measurement::CLUSTER_ID,
                pm25_concentration_measurement::attribute_id::MEASURED_VALUE,
                &Value::Float(value),
            ) if let Some(&Value::Uint(unit)) = unchanging_values.get(&AttributePath {
                endpoint,
                cluster: pm25_concentration_measurement::CLUSTER_ID,
                attribute: pm25_concentration_measurement::attribute_id::MEASUREMENT_UNIT,
            }) && let Some(unit) = MeasurementUnit::from_uint(unit) =>
            {
                Some(ClusterValueDetails {
                    name: "PM2.5",
                    value: ClusterValue::Float(value),
                    unit: Some(unit.short()),
                })
            }
            (
                carbon_dioxide_concentration_measurement::CLUSTER_ID,
                carbon_dioxide_concentration_measurement::attribute_id::MEASURED_VALUE,
                &Value::Float(value),
            ) if let Some(&Value::Uint(unit)) = unchanging_values.get(&AttributePath {
                endpoint,
                cluster: carbon_dioxide_concentration_measurement::CLUSTER_ID,
                attribute: carbon_dioxide_concentration_measurement::attribute_id::MEASUREMENT_UNIT,
            }) && let Some(unit) = MeasurementUnit::from_uint(unit) =>
            {
                Some(ClusterValueDetails {
                    name: "CO₂",
                    value: ClusterValue::Float(value),
                    unit: Some(unit.short()),
                })
            }
            _ => None,
        }
    }
}

impl Display for ClusterValueDetails {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.value)?;
        if let Some(unit) = &self.unit {
            write!(f, " {unit}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum ClusterValue {
    Boolean(bool),
    Float(f32),
}

impl ClusterValue {
    pub fn datatype_str(self) -> &'static str {
        match self {
            ClusterValue::Boolean(_) => "boolean",
            ClusterValue::Float(_) => "float",
        }
    }
}

impl Display for ClusterValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ClusterValue::Boolean(true) => f.write_str("on"),
            ClusterValue::Boolean(false) => f.write_str("off"),
            ClusterValue::Float(value) => write!(f, "{value}"),
        }
    }
}

impl From<ClusterValue> for influx_db_client::Value<'static> {
    fn from(value: ClusterValue) -> Self {
        match value {
            ClusterValue::Boolean(value) => influx_db_client::Value::Boolean(value),
            ClusterValue::Float(value) => influx_db_client::Value::Float(value.into()),
        }
    }
}

pub async fn read_all_known_clusters(
    node: &Node,
) -> Result<Vec<ClusterValueDetails>, matter_controller::Error> {
    let unchanging_values = node
        .read(UNCHANGING_ATTRIBUTES)
        .await?
        .into_iter()
        .collect::<HashMap<_, _>>();
    let changing_values = node.read(CHANGING_ATTRIBUTES).await?;

    Ok(changing_values
        .iter()
        .filter_map(|(path, value)| {
            ClusterValueDetails::for_attribute_value(path, value, &unchanging_values)
        })
        .collect())
}

/// The measurement unit attribute for a ConcentrationMeasurement cluster.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MeasurementUnit {
    Ppm = 0,
    Ppb = 1,
    Ppt = 2,
    Mgm3 = 3,
    Ugm3 = 4,
    Ngm3 = 5,
    Pm3 = 6,
    Bqm3 = 7,
}

impl MeasurementUnit {
    pub fn from_uint(value: u64) -> Option<Self> {
        match value {
            0 => Some(Self::Ppm),
            1 => Some(Self::Ppb),
            2 => Some(Self::Ppt),
            3 => Some(Self::Mgm3),
            4 => Some(Self::Ugm3),
            5 => Some(Self::Ngm3),
            6 => Some(Self::Pm3),
            7 => Some(Self::Bqm3),
            _ => None,
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Self::Ppm => "PPM",
            Self::Ppb => "PPB",
            Self::Ppt => "PPT",
            Self::Mgm3 => "mg/m³",
            Self::Ugm3 => "μg/m³",
            Self::Ngm3 => "ng/m³",
            Self::Pm3 => "P/m³",
            Self::Bqm3 => "Bq/m³",
        }
    }
}

impl From<MeasurementUnit> for u16 {
    fn from(value: MeasurementUnit) -> Self {
        value as _
    }
}

impl Display for MeasurementUnit {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.short())
    }
}
