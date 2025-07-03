use crate::db::Eui;
use crate::event::lora_gateway::GatewayStatus;
use crate::time::Timestamp;
use derive_new::new;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize, PartialEq, PartialOrd)]
#[serde(try_from = "u16", into = "u16")]
pub struct GatewayToken(u16);

impl GatewayToken {
    pub fn from_slice(u: &[u8]) -> Option<Self> {
        if u.len() != 2 {
            None
        } else {
            let mut a = [0; 2];
            a[0] = u[0];
            a[1] = u[1];
            Some(Self(u16::from_le_bytes(a)))
        }
    }
    pub fn as_bytes_token(&self) -> [u8; 2] {
        self.0.to_le_bytes()
    }

    pub fn random() -> Self {
        Self(rand::random())
    }
}

impl Debug for GatewayToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Display for GatewayToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl From<u16> for GatewayToken {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<GatewayToken> for u16 {
    fn from(value: GatewayToken) -> Self {
        value.0
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GatewayUpData {
    pub eui: Eui,
    pub version: u8,
    pub token: GatewayToken,
    pub time: Timestamp,
    pub source: GatewaySource,
    pub event: GatewayEventType,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GatewayUpDataHeader {
    pub eui: Eui,
    pub version: u8,
    pub token: GatewayToken,
    pub time: Timestamp,
    pub source: GatewaySource,
}
impl GatewayUpData {
    pub fn into_inner(self) -> (GatewayUpDataHeader, GatewayEventType) {
        let h = GatewayUpDataHeader {
            eui: self.eui,
            version: self.version,
            token: self.token,
            time: self.time,
            source: self.source,
        };
        (h, self.event)
    }
}

impl Debug for GatewayUpData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "gateway: {}, token: {}, event: {:?}", self.eui, self.token, self.event)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GatewaySource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<std::net::SocketAddr>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum GatewayEventType {
    Status(GatewayStatus),
    PushData(Vec<RXPK>),
    Pull,
    TxAck,
}

impl Debug for GatewayEventType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Status(_) => "Status",
            Self::PushData(_) => "PushData",
            Self::Pull => "Pull",
            Self::TxAck => "TxAck",
        };
        write!(f, "{}", s)
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub enum UpMode {
    LORA,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct RXPK {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    pub tmst: u32,
    pub freq: f32,
    pub chan: u32,
    pub rfch: u32,
    pub stat: i32,
    pub modu: UpMode,
    pub datr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codr: Option<String>,
    pub rssi: i32,
    pub lsnr: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
    pub data: String,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct TXPK {
    pub imme: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tmst: Option<u32>,
    #[serde(serialize_with = "serde_freq")]
    pub freq: f32,
    pub rfch: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub powe: Option<i32>,
    pub modu: UpMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codr: Option<String>,
    pub ipol: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u32>,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ncrc: Option<bool>,
}
fn serde_freq<S>(v: &f32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let rounded = (v * 1000.0).round() / 1000.0;
    serializer.serialize_f32(rounded)
}

#[derive(serde::Serialize, serde::Deserialize, new)]
pub struct DownStream {
    pub txpk: TXPK,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct GatewayDownData {
    pub eui: Eui,
    pub version: u8,
    pub token: GatewayToken,
    pub source: GatewaySource,
    pub time: Timestamp,
    pub pk: GatewayDownPK,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum GatewayDownPK {
    PullAck,
    PushAck,
    PullResponse(DownStream),
}
