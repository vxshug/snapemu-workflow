use arc_swap::ArcSwap;
use once_cell::sync::Lazy;
use serde::Deserialize;
use snap_config::SnapConfig;
use std::sync::Arc;
use tracing::info;
use crate::man::influxdb::InfluxDbClient;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub db: snap_config::DatabaseConfig,
    pub redis: snap_config::RedisConfig,
    #[serde(default)]
    pub log: snap_config::LogLevelConfig,
    pub jwt_key: String,
    #[serde(default)]
    pub concat_email: Option<String>,
    pub api: ApiConfig,
    pub device_data_timeout_day: Option<u32>,
    #[serde(default)]
    pub tsdb: Option<snap_config::TsdbConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            db: Default::default(),
            redis: Default::default(),
            log: Default::default(),
            jwt_key: "".to_string(),
            concat_email: None,
            tsdb: Default::default(),
            api: ApiConfig {
                predefine: None,
                oss: None,
                email: None,
                model: None,
                web_url: "".to_string(),
                mqtt_salt: "".to_string(),
                openapi: false,
                tracing: false,
                cors: false,
                host: "0.0.0.0".to_string(),
                port: 8080,
                eui_mask: _default_eui_mask(),
                grpc: _default_grpc(),
            },
            device_data_timeout_day: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PredefineConfig {
    #[serde(default)]
    pub device_url: Option<String>,
    #[serde(default)]
    pub device_auth: Option<String>,
    #[serde(default)]
    pub user_url: Option<String>,
    #[serde(default)]
    pub user_auth: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EmailConfig {
    pub server: String,
    pub user: String,
    pub password: String,
    pub sender: String,
    #[serde(default = "_default_email_port")]
    pub port: u32,
}

#[derive(Debug, Deserialize)]
pub struct AliyunOSSConfig {
    pub bucket: String,
    pub url: String,
    pub prefix: String,
    pub file_url: String,
    pub key: String,
    pub secret: String,
}

#[derive(Debug, Deserialize)]
pub struct ModelConfig {
    pub path: String,
}

fn _default_email_port() -> u32 {
    465
}

#[derive(Debug, Deserialize)]
pub struct ApiConfig {
    #[serde(default)]
    pub predefine: Option<PredefineConfig>,
    #[serde(default)]
    pub oss: Option<AliyunOSSConfig>,
    #[serde(default)]
    pub email: Option<EmailConfig>,
    #[serde(default)]
    pub model: Option<ModelConfig>,
    #[serde(default)]
    pub web_url: String,
    #[serde(default)]
    pub mqtt_salt: String,
    #[serde(default)]
    pub openapi: bool,
    #[serde(default)]
    pub tracing: bool,
    #[serde(default)]
    pub cors: bool,
    #[serde(default = "_default_host")]
    pub host: String,
    #[serde(default = "_default_port")]
    pub port: u16,
    #[serde(default = "_default_eui_mask")]
    pub eui_mask: u64,
    #[serde(default = "_default_grpc")]
    pub grpc: String,
}

fn _default_grpc() -> String {
    "http://localhost:5100".to_string()
}

fn _default_host() -> String {
    "localhost".to_string()
}

fn _default_port() -> u16 {
    8080
}

fn _default_eui_mask() -> u64 {
    u64::max_value()
}

static CONFIG: Lazy<ArcSwap<AppConfig>> =
    Lazy::new(|| ArcSwap::new(Arc::new(AppConfig::default())));
pub fn load_config() -> arc_swap::Guard<Arc<AppConfig>> {
    CONFIG.load()
}

pub fn store_config(config: String, env_prefix: String) -> arc_swap::Guard<Arc<AppConfig>> {
    if !std::path::Path::new(&config).exists() {
        eprintln!("not fount config file in {}", config);
        let config = SnapConfig::builder().env_prefix(&env_prefix).build().unwrap();
        CONFIG.store(Arc::new(config.into_local_config().unwrap()));
        return load_config();
    }
    let config = SnapConfig::builder().add_file(&config).env_prefix(&env_prefix).build().unwrap();
    CONFIG.store(Arc::new(config.into_local_config().unwrap()));
    load_config()
}

pub fn load_tsdb() -> InfluxDbClient {
    let config = load_config();
    let tsdb_config = config.tsdb.clone().unwrap();
    let client = influxdb2::Client::new(tsdb_config.host, tsdb_config.org, tsdb_config.token);
    InfluxDbClient::new(tsdb_config.bucket, client)
}


pub async fn load_db() -> sea_orm::DatabaseConnection {
    let config = load_config();
    let username = config.db.username.clone();
    let password = config.db.password.clone();
    let port = config.db.port;
    let count = config.db.connection_count;
    let db = config.db.db.clone();
    let host = config.db.host.clone();
    info!(event = "config", "type" = "db", host = host, "DB Config success");
    let url = format!("postgres://{username}:{password}@{host}:{port}/{db}");
    let mut option = sea_orm::ConnectOptions::new(url);
    option.max_connections(count as _);
    option.sqlx_logging(false);
    sea_orm::Database::connect(option).await.unwrap()
}
