use anyhow::{Context, bail};
use serde::Deserialize;
use std::{fs::read_to_string, net::SocketAddr, path::Path};

/// Paths at which to look for the config file. They are searched in order, and the first one that
/// exists is used.
const CONFIG_FILENAMES: [&str; 2] = ["matter-influx.toml", "/etc/matter-influx.toml"];

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The address on which the webserver should listen.
    #[serde(default = "default_webserver_address")]
    pub webserver_address: SocketAddr,
    /// The address on which the Matter Controller should listen.
    #[serde(default = "default_matter_controller_address")]
    pub matter_controller_address: SocketAddr,
    /// The Fabric ID to use for the Matter Controller.
    #[serde(default = "default_matter_fabric_id")]
    pub matter_fabric_id: u64,
    /// The directory in which to store Matter controller state.
    #[serde(default = "default_matter_data_path")]
    pub matter_data_path: String,
}

impl Config {
    pub fn from_file() -> Result<Self, anyhow::Error> {
        for filename in &CONFIG_FILENAMES {
            if Path::new(filename).is_file() {
                return Config::read(filename);
            }
        }
        bail!(
            "Unable to find config file in any of {:?}",
            &CONFIG_FILENAMES
        );
    }

    fn read(filename: &str) -> Result<Config, anyhow::Error> {
        let config_file =
            read_to_string(filename).with_context(|| format!("Reading {filename}"))?;
        Ok(toml::from_str(&config_file)?)
    }
}

fn default_webserver_address() -> SocketAddr {
    "[::]:3009".parse().unwrap()
}

fn default_matter_controller_address() -> SocketAddr {
    "[::]:3010".parse().unwrap()
}

fn default_matter_fabric_id() -> u64 {
    2000
}

fn default_matter_data_path() -> String {
    "matter-influx/matter/".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parsing the example config file should not give any errors.
    #[test]
    fn example_config() {
        Config::read("matter-influx.example.toml").unwrap();
    }

    /// Parsing an empty config file should not give any errors.
    #[test]
    fn empty_config() {
        toml::from_str::<Config>("").unwrap();
    }
}
