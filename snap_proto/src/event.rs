use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, ::prost::Message)]
pub struct LoRaNodeJoinRequest {
    #[prost(uint64, tag = "1")]
    pub app_eui: u64,
    #[prost(uint64, tag = "2")]
    pub dev_eui: u64,
    #[prost(int64, tag = "3")]
    pub time: i64,
}

#[derive(Serialize, Deserialize, Clone, ::prost::Message)]
pub struct LoRaNodeJoinAccept {
    #[prost(uint32, tag = "1")]
    pub dev_addr: u32,
    #[prost(int64, tag = "2")]
    pub time: i64,
}

#[derive(Serialize, Deserialize, Clone, ::prost::Message)]
pub struct LoRaNodeUplinkData {
    #[prost(uint32, tag = "1")]
    pub dev_addr: u32,
    #[prost(bool, tag = "2")]
    pub confirm: bool,
    #[prost(int32, tag = "3")]
    pub f_port: i32,
    #[prost(int32, tag = "4")]
    pub f_cnt: i32,
    #[prost(string, optional, tag = "5")]
    pub payload: Option<::prost::alloc::string::String>,
    #[prost(string, optional, tag = "6")]
    pub decoded_payload: Option<::prost::alloc::string::String>,
    #[prost(message, tag = "7")]
    pub gateway: Option<GatewayRxStatus>,
    #[prost(int64, tag = "8")]
    pub time: i64,
}

#[derive(Serialize, Deserialize, Clone, ::prost::Message)]
pub struct GatewayRxStatus {
    #[prost(uint64, tag = "1")]
    pub id: u64,
    #[prost(uint64, tag = "2")]
    pub eui: u64,
    #[prost(int64, tag = "3")]
    pub time: i64,
    #[prost(int32, tag = "4")]
    pub rssi: i32,
    #[prost(float, tag = "5")]
    pub snr: f32,
}

#[derive(Serialize, Deserialize, Clone, ::prost::Message)]
pub struct LoRaNodeDownLinkData {
    #[prost(bool, tag = "1")]
    pub confirm: bool,
    #[prost(int32, tag = "2")]
    pub f_port: i32,
    #[prost(string, optional, tag = "3")]
    pub bytes: Option<String>,
    #[prost(int64, tag = "4")]
    pub time: i64,
}

#[derive(Serialize, Deserialize, Clone, ::prost::Message)]
pub struct LoRaGatewayStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[prost(string, optional, tag = "1")]
    pub time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[prost(float, optional, tag = "2")]
    pub lati: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[prost(float, optional, tag = "3")]
    pub long: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[prost(int32, optional, tag = "4")]
    pub alti: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[prost(uint32, optional, tag = "5")]
    pub rxnb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[prost(uint32, optional, tag = "6")]
    pub rxok: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[prost(uint32, optional, tag = "7")]
    pub rwfw: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[prost(float, optional, tag = "8")]
    pub ackr: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[prost(uint32, optional, tag = "9")]
    pub dwnb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[prost(uint32, optional, tag = "10")]
    pub txnb: Option<u32>,
}