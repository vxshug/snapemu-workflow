use crate::db::{Eui, LoRaAddr};
use crate::Id;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, ::prost::Message)]
pub struct JoinRequest {
    #[prost(uint64, tag = "1")]
    pub app_eui: u64,
    #[prost(uint64, tag = "2")]
    pub dev_eui: u64,
    #[prost(int64, tag = "3")]
    pub time: i64,
}

#[derive(Serialize, Deserialize, Clone, ::prost::Message)]
pub struct JoinAccept {
    #[prost(uint32, tag = "1")]
    pub dev_addr: u32,
    #[prost(int64, tag = "2")]
    pub time: i64,
}

#[derive(Serialize, Deserialize, Clone, ::prost::Message)]
pub struct UplinkData {
    #[prost(uint32, tag = "1")]
    pub dev_addr: u32,
    #[prost(bool, tag = "2")]
    pub confirm: bool,
    #[prost(int32, tag = "3")]
    pub f_port: i32,
    #[prost(int32, tag = "4")]
    pub f_cnt: i32,
    #[prost(string, optional, tag = "5")]
    pub payload: Option<String>,
    #[prost(string, optional, tag = "6")]
    pub decoded_payload: Option<String>,
    #[prost(int64, tag = "7")]
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
pub struct DownLinkData {
    #[prost(bool, tag = "1")]
    pub confirm: bool,
    #[prost(int32, tag = "2")]
    pub f_port: i32,
    #[prost(string, optional, tag = "3")]
    pub bytes: Option<String>,
    #[prost(int64, tag = "4")]
    pub time: i64,
}

#[cfg(test)]
mod tests {}
