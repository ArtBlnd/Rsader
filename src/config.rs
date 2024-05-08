use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub bithumb: Option<BithumbConfig>,
    pub upbit: Option<UpbitConfig>,
    pub binance: Option<BinanceConfig>,
}

impl Config {
    pub fn get() -> &'static Config {
        static CONFIG: once_cell::sync::Lazy<Config> = once_cell::sync::Lazy::new(|| {
            let config = std::fs::read_to_string("config.toml").unwrap();
            toml::from_str(&config).unwrap()
        });

        &CONFIG
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BithumbConfig {
    pub connect_key: String,
    pub secret_key: String,
    pub ko_name: String,
    pub en_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpbitConfig {
    pub access_key: String,
    pub secret_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BinanceConfig {
    pub api_key: String,
    pub secret_key: String,
}
