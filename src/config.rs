use serde::{Deserialize, Serialize};
use std::{fs::read_to_string, sync::LazyLock};

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::load_config);

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub db: String,
    pub index_path: String,
    pub index_with_full_text: bool,
    pub addr: String,
    pub raw_data_path: Option<String>,
}

impl Config {
    fn load_config() -> Config {
        let cfg_file = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "config.toml".to_owned());
        let config: Config = basic_toml::from_str(&read_to_string(cfg_file).unwrap()).unwrap();
        config
    }
}
