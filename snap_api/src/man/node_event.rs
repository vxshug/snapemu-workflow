use crate::error::{ApiError, ApiResult};
use common_define::event::{DeviceEvent, DeviceEventType};
use common_define::Id;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tonic::{Response, Status, Streaming};
use tracing::warn;
use common_define::db::Eui;
use common_define::event::lora_gateway::{GatewayEvent, GatewayEventType, GatewayStatus};
use common_define::event::lora_node::{DownLinkData, GatewayRxStatus, JoinAccept, JoinRequest, UplinkData};
use common_define::time::Timestamp;
use crate::load::load_config;
use prost::Message;

pub struct NodeEvent {
    rx: broadcast::Receiver<DeviceEvent>,
}

impl NodeEvent {
    pub async fn event(&mut self) -> ApiResult<DeviceEvent> {
        self.rx.recv().await.map_err(|e| ApiError::User("channel close".into()))
    }

    pub fn into_stream(self) -> BroadcastStream<DeviceEvent> {
        BroadcastStream::new(self.rx)
    }
}

#[derive(Clone)]
pub struct NodeEventManager {
    map: Arc<Mutex<HashMap<Id, broadcast::Sender<DeviceEvent>>>>,
}

impl NodeEventManager {
    pub fn new() -> Self {
        Self { map: Default::default() }
    }

    pub(crate) fn subscribe(&self, device: Id) -> NodeEvent {
        let mut map = self.map.lock().unwrap();
        match map.get(&device) {
            None => {
                let (tx, rx) = broadcast::channel(100);
                map.insert(device, tx);
                NodeEvent { rx }
            }
            Some(tx) => NodeEvent { rx: tx.subscribe() },
        }
    }

    pub(crate) async fn start(&self) {
        loop {
            let server = snap_proto::manager::manager_client::ManagerClient::connect(load_config().api.grpc.clone()).await;
            if let Ok(mut server) = server {
                match server.logs(snap_proto::manager::LogRequest {}).await {
                    Ok(resp) => {
                        let mut resp = resp.into_inner();
                        while let Ok(Some(e)) = resp.message().await {
                            self.broadcast(e);
                        }
                    }
                    Err(e) => {
                        warn!("grpc error {:?}", e);
                    }
                };
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }

    pub(crate) fn broadcast(&self, event: snap_proto::manager::LogResponse) {
        let mut map = self.map.lock().unwrap();

        if let Some(identity) = event.identity {
            let device_id = Id(identity.id);
            if let Some(tx) = map.get(&device_id) {
                if let Ok(log_type) = snap_proto::manager::LogType::try_from(event.log_type) {
                    let device_event = match log_type {
                        snap_proto::manager::LogType::LoRaGatewayStatus => {
                            if let Ok(st) = snap_proto::event::LoRaGatewayStatus::decode_length_delimited(&*event.payload) {
                                Some(DeviceEvent {
                                    device: device_id,
                                    event: DeviceEventType::Gateway(GatewayEvent {
                                        eui: Eui(identity.eui),
                                        time: Timestamp::now(),
                                        gateway_event: GatewayEventType::Status(GatewayStatus {
                                            time: st.time,
                                            lati: st.lati,
                                            long: st.long,
                                            alti: st.alti,
                                            rxnb: st.rxnb,
                                            rxok: st.rxok,
                                            rwfw: st.rwfw,
                                            ackr: st.ackr,
                                            dwnb: st.dwnb,
                                            txnb: st.txnb,
                                        }),
                                    }),
                                })
                            } else { None }
                        }
                        snap_proto::manager::LogType::LoRaNodeJoinRequest => {
                            if let Ok(jr) = snap_proto::event::LoRaNodeJoinRequest::decode_length_delimited(&*event.payload) {
                                Some(DeviceEvent {
                                    device: device_id,
                                    event: DeviceEventType::JoinRequest(JoinRequest {
                                        dev_eui: jr.dev_eui,
                                        app_eui: jr.app_eui,
                                        time: jr.time,
                                    }),
                                })
                            } else { None }
                        }
                        snap_proto::manager::LogType::LoRaNodeJoinAccept => {
                            if let Ok(ja) = snap_proto::event::LoRaNodeJoinAccept::decode_length_delimited(&*event.payload) {
                                Some(DeviceEvent {
                                    device: device_id,
                                    event: DeviceEventType::JoinAccept(JoinAccept {
                                        dev_addr: ja.dev_addr,
                                        time: ja.time,
                                    }),
                                })
                            } else { None }
                        }
                        snap_proto::manager::LogType::LoRaNodeUplinkData => {
                            if let Ok(ud) = snap_proto::event::LoRaNodeUplinkData::decode_length_delimited(&*event.payload) {
                                Some(DeviceEvent {
                                    device: device_id,
                                    event: DeviceEventType::UplinkData(UplinkData {
                                        dev_addr: ud.dev_addr,
                                        confirm: ud.confirm,
                                        f_port: ud.f_port,
                                        f_cnt: ud.f_cnt,
                                        payload: ud.payload,
                                        decoded_payload: ud.decoded_payload,
                                        time: ud.time,
                                    }),
                                })
                            } else { None }
                        }
                        snap_proto::manager::LogType::LoRaNodeDownLinkData => {
                            if let Ok(dd) = snap_proto::event::LoRaNodeDownLinkData::decode_length_delimited(&*event.payload) {
                                Some(DeviceEvent {
                                    device: device_id,
                                    event: DeviceEventType::DownLinkData(DownLinkData {
                                        bytes: dd.bytes,
                                        confirm: dd.confirm,
                                        f_port: dd.f_port,
                                        time: dd.time,
                                    }),
                                })
                            } else { None }
                        }
                    };
                    if let Some(event) = device_event {
                        if let Err(e) = tx.send(event) {
                            map.remove(&e.0.device);
                        }
                    }
                }

            }
        }
    }
}
