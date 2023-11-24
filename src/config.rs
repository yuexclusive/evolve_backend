use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::fs;

static mut CONFIG: OnceCell<Config> = OnceCell::new();

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

pub fn cfg() -> &'static Config {
    let res = unsafe {
        CONFIG.get_or_init(|| {
            let config_path = "config.toml";
            let str = fs::read_to_string(config_path).unwrap();
            let config: Config = toml::from_str(&str).unwrap();
            config
        })
    };
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_config_name() {
        let name = &cfg().name;
        assert_eq!("evolve_backend", name)
    }
}
