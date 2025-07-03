use crate::db::Eui;
use crate::time::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct GatewayEvent {
    pub eui: Eui,
    pub time: Timestamp,
    pub gateway_event: GatewayEventType,
}


#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum GatewayEventType {
    Status(GatewayStatus),
    Join(JoinPayload),
    Data(DataPayload),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GatewayStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lati: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alti: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rxnb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rxok: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rwfw: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ackr: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dwnb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txnb: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JoinPayload {
    pub app_eui: Eui,
    pub dev_eui: Eui,
    pub dev_nonce: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DataPayload {
    pub payload: String,
    pub f_port: u16,
    pub f_cnt: u32,
    pub dev_addr: String,
    pub datr: String,
    pub codr: String,
    pub frequency: f32,
    pub rssi: i32,
    pub snr: i32,
    pub channel: i32,
}
