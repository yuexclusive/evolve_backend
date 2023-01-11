use serde::{Deserialize, Serialize};
use std::fs;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref CONFIG: Config = {
        let config_path = "config.toml";
        let str = fs::read_to_string(config_path).unwrap();
        let config: Config = toml::from_str(&str).unwrap();
        config
    };
}
#[derive(Deserialize, Serialize)]
pub struct Redis {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct MeiliSearch {
    pub address: String,
    pub api_key: String,
}

#[derive(Deserialize, Serialize)]
pub struct Email {
    pub username: String,
    pub password: String,
    pub relay: String,
    pub port: u16,
}

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub redis: Redis,
    pub meilisearch: MeiliSearch,
    pub email: Email,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_1() {
        let name = &CONFIG.name;
        assert_eq!("evolve_backend", name)
    }
}
