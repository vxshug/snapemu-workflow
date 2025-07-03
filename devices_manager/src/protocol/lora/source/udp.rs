use crate::load::load_config;
use crate::service::lorawan_gateway::gateway_event;
use crate::{DeviceError, DeviceResult};
use common_define::db::Eui;
use common_define::event::lora_gateway::GatewayStatus;
use common_define::lorawan_bridge::{
    DownStream, GatewayEventType, GatewaySource, GatewayToken, GatewayUpData, RXPK,
};
use common_define::time::Timestamp;
use serde::Deserialize;
use std::{net::SocketAddr, ops::Deref, sync::Arc};
use tokio::net::UdpSocket;
use tracing::instrument;
use tracing::{info, warn};

#[derive(Clone)]
pub struct UdpCli {
    socket: Arc<UdpSocket>,
}

pub async fn listen_udp() -> DeviceResult<(UdpForward, LoRaUdp)> {
    let config = load_config();
    let host = config.device.lorawan.host.clone();
    let port = config.device.lorawan.port;
    info!("udp listen: {}:{}", host, port);
    let socket = UdpSocket::bind(format!("{}:{}", host, port)).await?;
    let socket = UdpCli { socket: Arc::new(socket) };
    Ok((UdpForward { rx: socket.clone() }, LoRaUdp { socket }))
}

impl Deref for UdpCli {
    type Target = UdpSocket;
    fn deref(&self) -> &Self::Target {
        self.socket.as_ref()
    }
}

pub struct UdpForward {
    rx: UdpCli,
}

impl UdpForward {
    pub fn new(rx: UdpCli) -> Self {
        Self { rx }
    }

    #[instrument(skip(self), name = "udp")]
    pub async fn start(mut self) {
        let mut buffer = [0; 3096];
        loop {
            if let Ok((len, addr)) = self.rx.recv_from(&mut buffer).await {
                self.log(buffer[0..len].as_ref(), addr).await;
            }
        }
    }
    #[instrument(skip(self, s))]
    async fn log(&mut self, s: &[u8], addr: SocketAddr) {
        match self.decode(s, addr).await {
            Ok(up) => gateway_event(up),
            Err(e) => {
                warn!(addr = addr.to_string(), "{}", e);
            }
        }
    }
    #[instrument(skip(self, s))]
    async fn decode(&mut self, s: &[u8], addr: SocketAddr) -> DeviceResult<GatewayUpData> {
        if s.len() < 12 {
            return Err(DeviceError::Data(format!("gateway receive invalid: {:X?}", s)));
        }
        let version = s[0];
        if version != 2 {
            return Err(DeviceError::Data(format!("gateway receive invalid version: {:X?}", s)));
        }

        let event = match s[3] {
            0 => match split_push_and_state(&s[12..]) {
                Some(s) => s,
                None => {
                    return Err(DeviceError::Data(format!(
                        "gateway receive invalid payload: {:X?}",
                        s
                    )))
                }
            },
            2 => GatewayEventType::Pull,
            5 => GatewayEventType::TxAck,
            _ => {
                return Err(DeviceError::Data(format!(
                    "gateway receive invalid version: {:X?}",
                    s
                )));
            }
        };

        let token = GatewayToken::from_slice(&s[1..3])
            .ok_or(DeviceError::Data(format!("gateway receive invalid token: {:X?}", s)))?;
        let eui = Eui::from_be_bytes(&s[4..12])
            .ok_or(DeviceError::Data(format!("gateway receive invalid eui: {:X?}", s)))?;

        let data = GatewayUpData {
            eui,
            version,
            token,
            time: Timestamp::now(),
            source: GatewaySource { ip: Some(addr) },
            event,
        };
        Ok(data)
    }
}

#[derive(Deserialize)]
struct UpPack {
    rxpk: Option<Vec<RXPK>>,
    stat: Option<GatewayStatus>,
}

fn split_push_and_state(s: &[u8]) -> Option<GatewayEventType> {
    if let Ok(up) = serde_json::from_slice::<UpPack>(s) {
        if let Some(stat) = up.stat {
            return Some(GatewayEventType::Status(stat));
        }
        if let Some(rxpk) = up.rxpk {
            return Some(GatewayEventType::PushData(rxpk));
        }
    }
    warn!("invalid push data payload: {:?}", std::str::from_utf8(s));
    None
}

#[derive(Clone)]
pub struct LoRaUdp {
    socket: UdpCli,
}

impl LoRaUdp {
    pub fn new(socket: UdpCli) -> Self {
        Self { socket }
    }

    pub(crate) async fn down(
        &self,
        data: DownStream,
        version: u8,
        token: GatewayToken,
        addr: Option<SocketAddr>,
    ) -> DeviceResult {
        match addr {
            None => {
                warn!("missing download addr");
            }
            Some(o) => {
                let s = serde_json::to_vec(&data)?;
                let mut t = Vec::with_capacity(4 + s.len());
                t.push(version);
                t.extend_from_slice(&token.as_bytes_token());
                t.push(0x3);
                t.extend(s);
                self.socket.send_to(&t, o).await.map_err(DeviceError::warn)?;
            }
        }
        Ok(())
    }

    pub(crate) async fn push_ack(
        &self,
        version: u8,
        token: GatewayToken,
        addr: Option<SocketAddr>,
    ) -> DeviceResult {
        match addr {
            None => {
                warn!("missing download addr");
            }
            Some(addr) => {
                let token = token.as_bytes_token();
                let mut buf = [0; 4];
                buf[0] = version;
                buf[1] = token[0];
                buf[2] = token[1];
                buf[3] = 1;
                self.socket.send_to(&buf, addr).await?;
            }
        }
        Ok(())
    }
    pub(crate) async fn pull_ack(
        &self,
        version: u8,
        token: GatewayToken,
        eui: Eui,
        addr: Option<SocketAddr>,
    ) -> DeviceResult {
        match addr {
            None => {
                warn!("missing download addr");
            }
            Some(addr) => {
                let mut buf = [0; 12];
                buf[0] = version;
                let token = token.as_bytes_token();
                buf[1] = token[0];
                buf[2] = token[1];
                buf[3] = 4;
                let eui = eui.to_bytes();
                buf[4..12].copy_from_slice(eui.as_slice());
                self.socket.send_to(&buf, addr).await?;
            }
        }
        Ok(())
    }
}
