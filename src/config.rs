use anyhow::{Context, bail};
use influx_db_client::{Client, Url};
use serde::Deserialize;
use std::{
    fs::read_to_string,
    net::SocketAddr,
    path::{Path, PathBuf},
};

/// Paths at which to look for the config file. They are searched in order, and the first one that
/// exists is used.
const CONFIG_FILENAMES: [&str; 2] = ["matter-influx.toml", "/etc/matter-influx.toml"];

const DEFAULT_INFLUXDB_URL: &str = "http://localhost:8086";
const DEFAULT_DATABASE: &str = "matter";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The address on which the webserver should listen.
    #[serde(default = "default_webserver_address")]
    pub webserver_address: SocketAddr,
    /// The Fabric ID to use for the Matter Controller.
    #[serde(default = "default_matter_fabric_id")]
    pub matter_fabric_id: u64,
    /// The directory in which to store Matter controller state.
    #[serde(default = "default_matter_data_path")]
    pub matter_data_path: PathBuf,
    /// The directory of PAA root certificates.
    #[serde(default = "default_paa_dir")]
    pub paa_dir: PathBuf,
    /// The directory of CD certificates.
    #[serde(default = "default_cd_dir")]
    pub cd_dir: PathBuf,
    pub influxdb: Option<InfluxDbConfig>,
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

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InfluxDbConfig {
    pub url: Url,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: String,
}

impl Default for InfluxDbConfig {
    fn default() -> InfluxDbConfig {
        InfluxDbConfig {
            url: DEFAULT_INFLUXDB_URL.parse().unwrap(),
            username: None,
            password: None,
            database: DEFAULT_DATABASE.to_owned(),
        }
    }
}

impl InfluxDbConfig {
    /// Construct a new InfluxDB [`Client`] based on the configuration options.
    pub fn make_client(&self) -> Result<Client, anyhow::Error> {
        let mut influxdb_client = Client::new(self.url.to_owned(), &self.database);
        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            influxdb_client = influxdb_client.set_authentication(username, password);
        }
        Ok(influxdb_client)
    }
}

fn default_webserver_address() -> SocketAddr {
    "[::]:3009".parse().unwrap()
}

fn default_matter_fabric_id() -> u64 {
    2000
}

fn default_matter_data_path() -> PathBuf {
    "matter-influx/controller-state.bin".into()
}

fn default_paa_dir() -> PathBuf {
    "/usr/share/matter-influx/paa-root-certs".into()
}

fn default_cd_dir() -> PathBuf {
    "/usr/share/matter-influx/cd-certs".into()
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
