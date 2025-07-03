use serde::{Deserialize, Serialize};


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GwConfig {
    #[serde(rename = "SX130x_conf")]
    pub sx130x_conf: serde_json::Value,
    #[serde(rename = "gateway_conf")]
    pub gateway_conf: GatewayConf,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayConf {
    #[serde(rename = "gateway_ID")]
    pub gateway_id: String,
    #[serde(rename = "server_address")]
    pub server_address: String,
    #[serde(rename = "serv_port_up")]
    pub serv_port_up: i64,
    #[serde(rename = "serv_port_down")]
    pub serv_port_down: i64,
    #[serde(rename = "keepalive_interval")]
    pub keepalive_interval: i64,
    #[serde(rename = "stat_interval")]
    pub stat_interval: i64,
    #[serde(rename = "push_timeout_ms")]
    pub push_timeout_ms: i64,
    #[serde(rename = "forward_crc_valid")]
    pub forward_crc_valid: bool,
    #[serde(rename = "forward_crc_error")]
    pub forward_crc_error: bool,
    #[serde(rename = "forward_crc_disabled")]
    pub forward_crc_disabled: bool,
}