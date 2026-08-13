use log::debug;
use matter_controller::{AttributePath, Node, Value};
use std::{
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
};

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
