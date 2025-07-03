use crate::event::lora_gateway::GatewayEvent;
use crate::event::lora_node::{DownLinkData, JoinAccept, JoinRequest, UplinkData};
use crate::Id;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

mod log;
pub mod lora_gateway;
pub mod lora_node;
use crate::db::Eui;
pub use log::PlatformLog;

#[derive(Serialize, Deserialize, Clone)]
pub struct DeviceEvent {
    pub device: Id,
    pub event: DeviceEventType,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "event")]
pub enum DeviceEventType {
    JoinRequest(JoinRequest),
    JoinAccept(JoinAccept),
    UplinkData(UplinkData),
    DownLinkData(DownLinkData),
    Gateway(GatewayEvent),
    SnapDevice(SnapEvent),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SnapEvent {
    pub eui: Eui,
    pub data: Vec<u8>,
}


#[derive(Serialize, Deserialize, Clone)]

pub struct DownloadMessage {
    pub eui: Eui,
    pub port: u8,
    pub data: String,
}

impl DeviceEvent {
    pub const KAFKA_TOPIC: &'static str = "LoRaNode-Event";
    pub const DOWN_TOPIC: &'static str = "Device-Downlink";
}
