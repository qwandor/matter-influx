use log::debug;
use matter_clusters::r#gen::{on_off, relative_humidity_measurement, temperature_measurement};
use matter_controller::{AttributePath, Node, ReadPath, Value};
use std::{
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
};

const CLUSTER_ID_PM2_5_CONCENTRATION_MEASUREMENT: u32 = 0x042a;
const CLUSTER_ID_CARBON_DIOXIDE_CONCENTRATION_MEASUREMENT: u32 = 0x040d;
const CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREDVALUE: u32 = 0x0000;
const CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREMENTUNIT: u32 = 0x0008;

/// The value read from some cluster and parsed, ready to display.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct ClusterValueDetails {
    pub name: &'static str,
    pub value: ClusterValue,
    pub unit: Option<&'static str>,
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

impl Display for ClusterValue {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ClusterValue::Boolean(true) => f.write_str("on"),
            ClusterValue::Boolean(false) => f.write_str("off"),
            ClusterValue::Float(value) => write!(f, "{value}"),
        }
    }
}

pub async fn read_all_known_clusters(
    node: &Node,
    endpoint: u16,
) -> Result<Vec<ClusterValueDetails>, matter_controller::Error> {
    let mut cluster_values = Vec::new();

    if let &[(_, Value::Bool(on))] = node
        .read(&[ReadPath::concrete(
            endpoint,
            on_off::CLUSTER_ID,
            on_off::attribute_id::ON_OFF,
        )])
        .await?
        .as_slice()
    {
        cluster_values.push(ClusterValueDetails {
            name: "On",
            value: ClusterValue::Boolean(on),
            unit: None,
        });
    }

    if let &[(_, Value::Int(value))] = node
        .read(&[ReadPath::concrete(
            endpoint,
            temperature_measurement::CLUSTER_ID,
            temperature_measurement::attribute_id::MEASURED_VALUE,
        )])
        .await?
        .as_slice()
    {
        cluster_values.push(ClusterValueDetails {
            name: "Temperature",
            value: ClusterValue::Float(value as f32 / 100.0),
            unit: Some("°C"),
        });
    }

    if let &[(_, Value::Uint(value))] = node
        .read(&[ReadPath::concrete(
            endpoint,
            relative_humidity_measurement::CLUSTER_ID,
            relative_humidity_measurement::attribute_id::MEASURED_VALUE,
        )])
        .await?
        .as_slice()
    {
        cluster_values.push(ClusterValueDetails {
            name: "Temperature",
            value: ClusterValue::Float(value as f32 / 100.0),
            unit: Some("%"),
        });
    }

    if let &[Some(Value::Float(value)), Some(Value::Uint(unit))] = read_values_in_order(
        &node,
        &[
            AttributePath {
                endpoint: endpoint,
                cluster: CLUSTER_ID_PM2_5_CONCENTRATION_MEASUREMENT,
                attribute: CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREDVALUE,
            },
            AttributePath {
                endpoint: endpoint,
                cluster: CLUSTER_ID_PM2_5_CONCENTRATION_MEASUREMENT,
                attribute: CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREMENTUNIT,
            },
        ],
    )
    .await?
    .as_slice()
        && let Some(unit) = MeasurementUnit::from_uint(unit)
    {
        cluster_values.push(ClusterValueDetails {
            name: "PM2.5",
            value: ClusterValue::Float(value),
            unit: Some(unit.short()),
        });
    }

    if let &[Some(Value::Float(value)), Some(Value::Uint(unit))] = read_values_in_order(
        &node,
        &[
            AttributePath {
                endpoint: endpoint,
                cluster: CLUSTER_ID_CARBON_DIOXIDE_CONCENTRATION_MEASUREMENT,
                attribute: CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREDVALUE,
            },
            AttributePath {
                endpoint: endpoint,
                cluster: CLUSTER_ID_CARBON_DIOXIDE_CONCENTRATION_MEASUREMENT,
                attribute: CONCENTRATION_MEASUREMENT_ATTR_ID_MEASUREMENTUNIT,
            },
        ],
    )
    .await?
    .as_slice()
        && let Some(unit) = MeasurementUnit::from_uint(unit)
    {
        cluster_values.push(ClusterValueDetails {
            name: "CO₂",
            value: ClusterValue::Float(value),
            unit: Some(unit.short()),
        });
    }

    Ok(cluster_values)
}

pub async fn read_values_in_order(
    node: &Node,
    paths: &[AttributePath],
) -> Result<Vec<Option<Value>>, matter_controller::Error> {
    let read_paths = paths
        .into_iter()
        .map(|path| (*path).into())
        .collect::<Vec<_>>();
    let mut values = node
        .read(&read_paths)
        .await?
        .into_iter()
        .map(
            |(
                AttributePath {
                    endpoint,
                    cluster,
                    attribute,
                },
                value,
            )| ((endpoint, cluster, attribute), value),
        )
        .collect::<BTreeMap<_, _>>();
    debug!("values: {values:?}");
    Ok(paths
        .into_iter()
        .map(
            |&AttributePath {
                 endpoint,
                 cluster,
                 attribute,
                 ..
             }| values.remove(&(endpoint, cluster, attribute)),
        )
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
