#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use crate::decode::JsManager;
use crate::load::{load_config, store_config, State};
use crate::man::data::{DataError, DownloadDataCache, DownloadResponse};
use crate::man::mqtt::MessageProcessor;
use crate::man::redis_client::{RedisClient, RedisRecv};
use crate::man::{DecodeManager, DownlinkManager, Id, MQ};
use crate::service::custom_gateway::start_process_snap;
use common_define::event::DeviceEvent;
use once_cell::sync::Lazy;
use tracing::{error, info, warn};
use crate::man::influxdb::InfluxError;
use crate::mqtt::mqtt_auth_fn;
use crate::protocol::mqtt::{Broker, ConnectionSettings, ListenConfig, MqttConfig, RouterConfig, TlsConfig};

pub(crate) mod decode;
pub(crate) mod load;
pub(crate) mod man;
pub(crate) mod mqtt;
pub(crate) mod protocol;
pub(crate) mod service;

pub(crate) mod event;
pub(crate) mod integration;
pub(crate) mod db;

tokio::task_local! {
    static DEVICE_ID: Id;
}

fn device_id() -> Id {
    DEVICE_ID.try_with(|id| *id).unwrap_or_else(|_| Id::new(1))
}

#[derive(thiserror::Error, Debug)]
enum DeviceError {
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Data(String),
    #[error("{0}")]
    Device(String),
    #[error("{0}")]
    Warn(String),
    #[error("Register: {0}")]
    Register(String),
    #[error("{0}")]
    Base64(#[from] base64::DecodeError),
    #[error("redis error {0}")]
    Redis(redis::RedisError),
    #[error("redis pool {0}")]
    RedisPool(deadpool::managed::PoolError<deadpool_redis::redis::RedisError>),
    #[error("hex error {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("Io error {0}")]
    IO(#[from] std::io::Error),
    #[error("db error {0}")]
    Orm(sea_orm::DbErr),
    #[error("{0}")]
    Error(String),
    #[error("{0}")]
    Connect(String),
    #[error("Empty")]
    Empty,
    #[error("DataError {0}")]
    DataError(#[from] common_define::db::DataError),
    #[error("RequestError {0}")]
    RequestError(#[from] influxdb2::RequestError),
    #[error("MQTT error {0}")]
    InfluxError(#[from] InfluxError)
}

impl DeviceError {
    fn device<T: ToString>(msg: T) -> Self {
        let msg = msg.to_string();
        warn!("device {}", msg);
        Self::Device(msg)
    }
    fn data<T: ToString>(msg: T) -> Self {
        Self::Device(msg.to_string())
    }
    fn warn<T: ToString>(msg: T) -> Self {
        Self::Warn(msg.to_string())
    }
}

impl From<redis::RedisError> for DeviceError {
    fn from(value: redis::RedisError) -> Self {
        warn!("redis: {}", value);
        Self::Redis(value)
    }
}

impl From<DataError> for DeviceError {
    fn from(value: DataError) -> Self {
        Self::Data(value.msg)
    }
}

impl From<deadpool::managed::PoolError<deadpool_redis::redis::RedisError>> for DeviceError {
    fn from(value: deadpool::managed::PoolError<deadpool_redis::redis::RedisError>) -> Self {
        warn!("redis pool: {}", value);
        Self::RedisPool(value)
    }
}

impl From<sea_orm::DbErr> for DeviceError {
    fn from(value: sea_orm::DbErr) -> Self {
        warn!("db error: {}", value);
        Self::Orm(value)
    }
}

impl From<()> for DeviceError {
    fn from(_value: ()) -> Self {
        Self::Empty
    }
}

type DeviceResult<T = ()> = Result<T, DeviceError>;

static GLOBAL_TOPIC: Lazy<Topic> = Lazy::new(load::load_topic);

static GLOBAL_DEPEND: Lazy<DecodeManager> = Lazy::new(|| {
    let rt = JsManager::new();
    DecodeManager::new(rt.clone())
});

static GLOBAL_DOWNLOAD: Lazy<DownloadDataCache> = Lazy::new(DownloadDataCache::default);
static GLOBAL_DOWNLOAD_RESPONSE: Lazy<DownloadResponse> = Lazy::new(DownloadResponse::default);

static GLOBAL_STATE: Lazy<State> = Lazy::new(load::load_state);

static MODEL_MAP: Lazy<snap_model::ModelMap> = Lazy::new(|| {
    let config = load_config();
    snap_model::load_model_file(config.device.model.as_ref().map(|p| p.path.as_str()))
});

struct Topic {
    data: &'static str,
    gate_event: &'static str,
    down: &'static str,
}

pub async fn run(config: String, env_prefix: String) {
    let config = store_config(config, env_prefix);
    snap_config::init_logging(config.log);
    GLOBAL_STATE.db.ping().await.unwrap();


    let redis_client = RedisClient::get_client();
    let recv = RedisRecv::new(redis_client.get_pubsub().await.unwrap());

    MQ::new(recv).await.start().await
}
