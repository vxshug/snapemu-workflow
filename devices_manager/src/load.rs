use arc_swap::ArcSwap;
use once_cell::sync::Lazy;
use serde::Deserialize;
use snap_config::{DeviceTopicConfig, SnapConfig};
use std::sync::Arc;
use std::thread;
use std::thread::Thread;
use tracing::info;
use common_define::event::DeviceEvent;
use crate::event::{DeviceEventSender, DeviceManagerServer};
use crate::man::DownlinkManager;
use crate::man::influxdb::InfluxDbClient;
use crate::man::mqtt::{MessageProcessor, MqPublisher};
use crate::man::redis_client::{RedisClient, RedisRecv};
use crate::mqtt::mqtt_auth_fn;
use crate::protocol::lora::source::{listen_udp, LoRaUdp};
use crate::protocol::mqtt;
use crate::protocol::mqtt::Broker;
use crate::service::custom_gateway::start_process_snap;
use crate::Topic;

static CONFIG: Lazy<ArcSwap<DeviceConfig>> =
    Lazy::new(|| ArcSwap::new(Arc::new(DeviceConfig::default())));

pub fn store_config(config: String, env_prefix: String) -> arc_swap::Guard<Arc<DeviceConfig>> {
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
pub fn load_config() -> arc_swap::Guard<Arc<DeviceConfig>> {
    CONFIG.load()
}

#[derive(Deserialize, Debug, Default)]
pub struct DeviceConfig {
    #[serde(default)]
    pub db: snap_config::DatabaseConfig,
    #[serde(default)]
    pub redis: snap_config::RedisConfig,
    #[serde(default)]
    pub log: snap_config::LogLevelConfig,
    #[serde(default)]
    pub device: DeviceConfigInner,
    #[serde(default)]
    pub mqtt: Option<mqtt::MqttConfig>,
    #[serde(default)]
    pub snap: Option<SnapDeviceConfig>,
    #[serde(default)]
    pub tsdb: Option<snap_config::TsdbConfig>,
}

#[derive(Deserialize, Debug, Default)]
pub struct DeviceConfigInner {
    pub topic: DeviceTopicConfig,
    pub lorawan: LoRaConfig,
    pub model: Option<ModelConfig>,
    #[serde(default = "_default_rpc_port")]
    pub rpc_port: u16,
}

fn _default_rpc_port() -> u16 {
    5100
}

#[derive(Debug, Deserialize)]
pub struct ModelConfig {
    pub path: String,
}

#[derive(Deserialize, Debug)]
pub struct LoRaConfig {
    #[serde(default = "_default_lora_host")]
    pub host: String,
    #[serde(default = "_default_lora_port")]
    pub port: u16,
}

impl Default for LoRaConfig {
    fn default() -> Self {
        Self { host: _default_lora_host(), port: _default_lora_port() }
    }
}

fn _default_lora_host() -> String {
    "localhost".to_string()
}
fn _default_lora_port() -> u16 {
    1700
}

#[derive(Deserialize, Debug)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub client: String,
    #[serde(default)]
    pub ca: Option<String>,
    #[serde(default)]
    pub tls: bool,
    #[serde(default)]
    pub topic: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
pub struct SnapDeviceConfig {
    pub mqtt: MqttConfig,
}

pub struct State {
    pub db: sea_orm::DatabaseConnection,
    pub tsdb: InfluxDbClient,
    pub udp: LoRaUdp,
    pub mq: MqPublisher,
    pub event: DeviceEventSender
}
pub(crate) fn load_state() -> State {
    tokio::task::block_in_place(move || {
        tokio::runtime::Handle::current().block_on(async move {
            let db = load_db().await;
            let (forward, udp) = listen_udp().await.unwrap();
            tokio::spawn(async move {
                forward.start().await;
            });
            let tsdb = load_tsdb();
            let mut broker_config = load_config().mqtt.clone().unwrap();
            broker_config.external_auth = Some(Arc::new(mqtt_auth_fn));
            let mut b = Broker::new(broker_config);

            let (mut link_tx, link_rx) = b.link("client").unwrap();
            link_tx.subscribe("#").unwrap();

            let (download_tx, download_rx) = tokio::sync::mpsc::channel(1000);

            let redis_client = RedisClient::get_client();
            let mut consumer = RedisRecv::new(redis_client.get_pubsub().await.unwrap());
            consumer.subscribe(DeviceEvent::DOWN_TOPIC).await.unwrap();
            tokio::spawn(async move {
                DownlinkManager::new(download_rx).start_downlink().await;
            });

            let (tx, rx) = tokio::sync::mpsc::channel(100);
            let subscriber = MessageProcessor::new_with_sender(link_rx, tx, download_tx);
            tokio::spawn(subscriber.start());
            tokio::spawn(start_process_snap(rx));
            let broker_thread = thread::Builder::new().name("broker".to_string());
            broker_thread.spawn(move || {
                b.start().unwrap();
            }).unwrap();
            let (event_tx, _event_rx) = tokio::sync::broadcast::channel(100);
            let server = DeviceManagerServer::new(event_tx.clone());
            let rpc_port = load_config().device.rpc_port;
            let addr = format!("0.0.0.0:{rpc_port}").parse().unwrap();
            info!("grpc in {:?}", addr);
            tokio::spawn(tonic::transport::Server::builder()
                                 .add_service(snap_proto::manager::manager_server::ManagerServer::new(server))
                                 .serve(addr));
            let event = DeviceEventSender::new(event_tx);
            State { db, udp, tsdb, mq: MqPublisher::new(link_tx), event }
        })
    })
}

fn load_tsdb() -> InfluxDbClient {
    let config = load_config();
    let tsdb_config = config.tsdb.clone().unwrap();
    let client = influxdb2::Client::new(tsdb_config.host, tsdb_config.org, tsdb_config.token);
    InfluxDbClient::new(tsdb_config.bucket, client)
}

async fn load_db() -> sea_orm::DatabaseConnection {
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
    option.sqlx_logging(config.db.sqlx_logging);
    sea_orm::Database::connect(option).await.unwrap()
}

pub(crate) fn load_topic() -> Topic {
    let config = load_config();
    let data = config.device.topic.data.clone();
    let event = config.device.topic.event.clone();
    let down = config.device.topic.down.clone();

    Topic {
        data: Box::leak(data.into_boxed_str()),
        gate_event: Box::leak(event.into_boxed_str()),
        down: Box::leak(down.into_boxed_str()),
    }
}
