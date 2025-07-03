use common_define::lora::{LoRaJoinType, LoRaRegion};
use common_define::product::DeviceType;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LoRaParameter {
    pub app_skey: String,
    pub nwk_skey: String,
    pub dev_addr: String,
    pub app_key: String,
    pub app_eui: String,
    pub dev_eui: String,

    pub class_b: bool,
    pub class_c: bool,
    pub adr: bool,
    pub rx1_delay: i16,
    pub rx1_dro: i16,

    pub rx2_dr: i16,

    pub rx2_freq: i32,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LoRaNode {
    pub join_type: LoRaJoinType,
    pub sensor: String,
    pub region: LoRaRegion,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub blue_name: Option<String>,
    pub join_parameter: LoRaParameter,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LoRaGate {
    pub eui: String,
    pub region: LoRaRegion,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum DeviceParameter {
    Device(LoRaNode),
    Gate(LoRaGate),
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct PredefineDeviceInfo {
    pub name: String,
    pub eui: String,
    pub dev_addr: String,
    pub device_type: DeviceType,
    pub parameter: DeviceParameter,
}
