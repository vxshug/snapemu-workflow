use crate::DeviceResult;
use common_define::db::Eui;
use common_define::lorawan_bridge::RXPK;
use common_define::time::Timestamp;

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct PushState {
    pub(crate) eui: Eui,
    pub(crate) token: u16,
    pub(crate) version: u8,
    pub(crate) ip: String,
    pub(crate) time: Timestamp,
    pub(crate) event: GatewayEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) state: Option<GateWayState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<Vec<RXPK>>,
}
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) enum GatewayEvent {
    PushData,
    PullData,
    TXAck,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub(crate) struct GateWayState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) lati: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) long: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) alti: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rxnb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rxok: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rwfw: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ackr: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) dwnb: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) txnb: Option<u32>,
}

pub(crate) async fn gateway_status(state: PushState) -> DeviceResult {
    // crate::event::gateway::GatewayEvent::gateway_state(state).await?;
    Ok(())
}
