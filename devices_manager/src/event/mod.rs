pub mod gateway;

mod platform;

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::task::{ready, Context, Poll};
use std::time::Duration;
use crate::man::data::DownloadData;
use crate::man::lora::{LoRaGate, LoRaNode, LoRaNodeManager};
use crate::protocol::lora::payload::LoRaPayload;
use crate::service::lorawan_node::PushData;
use base64::Engine;
use common_define::db::{Eui, LoRaAddr};
use device_info::lorawan::NodeInfo;
use lorawan::parser::{AsPhyPayloadBytes, DataHeader};
use once_cell::sync::Lazy;
use prost::Message;
use redis::AsyncCommands;
use tokio::sync::oneshot;
use tonic::{Request, Response, Status};
use tracing::{info, warn};
use common_define::{lorawan_bridge, Id};
use common_define::lorawan_bridge::GatewayUpData;
use device_info::snap::SnapDeviceInfo;
use snap_proto::manager::{DownloadMessage, DownloadResponse, LogRequest, LogResponse, DeviceIdentity, LogType, GwConfigRequest, GwConfig, UpdateConfigRequest, ResultStatus};
use utils::base64::EncodeBase64;
use crate::{DeviceError, GLOBAL_STATE};
use crate::man::gw::{DataWrapper, GwCmd};
use crate::man::redis_client::RedisClient;
use crate::protocol::snap::DownJson;

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error(transparent)]
    Send(#[from] tokio::sync::broadcast::error::SendError<LogResponse>)
}
pub struct DeviceEventSender {
    tx: tokio::sync::broadcast::Sender<Result<LogResponse, tonic::Status>>
}

pub struct BroadcastStreamWrapper<T>(tokio_stream::wrappers::BroadcastStream<T>);

impl<T: 'static + Clone + Send> tokio_stream::Stream for BroadcastStreamWrapper<T> {
    type Item = T;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = Pin::new(&mut self.get_mut().0);
        let res = ready!(this.poll_next(cx));
        match res {
            Some(Ok(res)) => Poll::Ready(Some(res)),
            Some(Err(_e)) => Poll::Pending,
            None => Poll::Ready(None)
        }

    }
}

pub struct DeviceManagerServer {
    tx: tokio::sync::broadcast::Sender<Result<LogResponse, tonic::Status>>
}

impl DeviceManagerServer {
    pub fn new(tx: tokio::sync::broadcast::Sender<Result<LogResponse, tonic::Status>>) -> DeviceManagerServer {
        Self {
            tx
        }
    }
}

static CONFIG_CACHE: Lazy<Mutex<HashMap<Id, HashMap<u32, oneshot::Sender<GwConfig>>>>> = Lazy::new(||Mutex::new(HashMap::new()));

pub fn config_cache(id: &Id, key: u32) -> Option<oneshot::Sender<GwConfig>> {
    CONFIG_CACHE.lock().unwrap().get_mut(&id)
        .and_then(|ls| ls.remove(&key))
}

fn next_id() -> u32 {
    static ID: AtomicU32 = AtomicU32::new(0);
    ID.fetch_add(1, Ordering::Relaxed)
}

#[tonic::async_trait]
impl snap_proto::manager::manager_server::Manager for DeviceManagerServer {

    type LogsStream = BroadcastStreamWrapper<Result<LogResponse, tonic::Status>>;
    async fn logs(&self, _request: tonic::Request<LogRequest>) -> Result<tonic::Response<Self::LogsStream>, tonic::Status> {
        Ok(tonic::Response::new(BroadcastStreamWrapper(tokio_stream::wrappers::BroadcastStream::new(self.tx.subscribe()))))
    }

    async fn download(&self, request: tonic::Request<DownloadMessage>) -> Result<tonic::Response<DownloadResponse>, tonic::Status> {
        let request = request.into_inner();
        if let Some(identity) = request.identity {
            let device_id = Id(identity.id);
            let eui = Eui(identity.eui);
            if let Some(node) = LoRaNodeManager::get_node_by_eui(eui).await.map_err(|_e| Status::internal("get node device error"))? {
                let data =
                    base64::engine::general_purpose::STANDARD.decode(request.message.as_slice()).map_err(|_e| Status::internal("base64"))?;
                info!("{}, down message", eui);
                tokio::spawn(async move {
                    if let Err(e) = node
                        .dispatch_task_now(DownloadData::new_data_with_id(
                            data,
                            1,
                            request.port as _,
                        ))
                        .await
                    {
                        info!(device = eui.to_string(), "forward error: {}", e);
                    }
                });
                return Ok(Response::new(DownloadResponse {
                    identity: Some(identity),
                    result: ResultStatus::Ok as _ ,
                    message: "Ok".to_owned()
                }))
            }
            let mut conn = RedisClient::get_client().get_multiplexed_conn().await.map_err(|_e| Status::internal("get redis error"))?;

            if let Some(snap) = SnapDeviceInfo::load(eui, &mut conn).await.map_err(|_e| Status::internal("get snap device error"))? {
                if let Some(down_link) = snap.down {
                    if snap.freq.is_none() {
                        return Err(tonic::Status::invalid_argument("snap device not found freq"));
                    }
                    match crate::protocol::snap::DownloadData::new_with_eui(eui)
                        .set_ack()
                        .set_counter((snap.up_count + 1) as u16)
                        .encode_payload(&[], &snap.key)
                    {
                        Ok(o) => {
                            let j = DownJson {
                                token: rand::random(),
                                freq: snap.freq.unwrap(),
                                data: o,
                            };
                        }
                        Err(e) => {
                            warn!("failed to encode ack: {:?}", e);
                        }
                    }
                }
            }
        }
        Ok(Response::new(DownloadResponse {
            identity: request.identity,
            result: ResultStatus::Ok as _ ,
            message: "Ok".to_owned()
        }))
    }
    async fn config(&self, request: Request<GwConfigRequest>) -> Result<Response<GwConfig>, Status> {
        let request = request.into_inner();
        let user_id = Id(request.user_id);
        if let Some(identity) = request.identity {
            let device_id = Id(identity.id);
            let (tx, rx) = oneshot::channel();
            let key = next_id();
            if CONFIG_CACHE.lock().unwrap().contains_key(&device_id) {
                CONFIG_CACHE.lock().unwrap().get_mut(&device_id).unwrap().insert(key, tx);
            } else {
                let mut s = HashMap::new();
                s.insert(key, tx);
                CONFIG_CACHE.lock().unwrap().insert(device_id, s);
            };
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                CONFIG_CACHE.lock().unwrap().get_mut(&device_id).and_then(|map| {map.remove(&key)});
            });
            let eui = Eui(identity.eui);
            let topic = format!("user/{}/gw/{}/down", user_id, eui);
            let cmd = GwCmd::Config(DataWrapper::new(key,()));
            let payload = serde_json::to_string(&cmd).map_err(|e| Status::internal(e.to_string()))?;
            GLOBAL_STATE.mq.publish(crate::integration::mqtt::MqttMessage::new(payload, topic)).await;
            let config = tokio::time::timeout(Duration::from_secs(5), rx).await
                .map_err(|e| Status::already_exists("request timeout"))?
                .map_err(|e| Status::internal(e.to_string()))?;
            CONFIG_CACHE.lock().unwrap().remove(&device_id);
            Ok(Response::new(config))
        } else { Err(Status::not_found("Identity not found")) }
    }

    async fn update_config(&self, request: Request<UpdateConfigRequest>) -> Result<Response<GwConfig>, Status> {
        let request = request.into_inner();
        let user_id = Id(request.user_id);
        if let (Some(identity), Some(config) ) = (request.identity, request.config) {
            let device_id = Id(identity.id);
            let (tx, rx) = oneshot::channel();
            let key = next_id();
            if CONFIG_CACHE.lock().unwrap().contains_key(&device_id) {
                CONFIG_CACHE.lock().unwrap().get_mut(&device_id).unwrap().insert(key, tx);
            } else {
                let mut s = HashMap::new();
                s.insert(key, tx);
                CONFIG_CACHE.lock().unwrap().insert(device_id, s);
            };
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;
                CONFIG_CACHE.lock().unwrap().get_mut(&device_id).and_then(|map| {map.remove(&key)});
            });
            let eui = Eui(identity.eui);
            let topic = format!("user/{}/gw/{}/down", user_id, eui);
            let cmd = GwCmd::UpdateConfig(DataWrapper::new(key, config));
            let payload = serde_json::to_string(&cmd).map_err(|e| Status::internal(e.to_string()))?;
            GLOBAL_STATE.mq.publish(crate::integration::mqtt::MqttMessage::new(payload, topic)).await;
            let config = tokio::time::timeout(Duration::from_secs(5), rx).await
                .map_err(|e| Status::already_exists("request timeout"))?
                .map_err(|e| Status::internal(e.to_string()))?;
            Ok(Response::new(config))
        } else {
            Err(Status::internal("Invalid UpdateConfigRequest"))
        }
    }
}

impl DeviceEventSender {

    pub fn new(tx: tokio::sync::broadcast::Sender<Result<LogResponse, tonic::Status>>) -> Self {
        Self { tx }
    }
    pub(crate) async fn lora_gateway_status(
        &self,
        gate: &LoRaGate,
        state: GatewayUpData
    )  {
        if let lorawan_bridge::GatewayEventType::Status(st)  = state.event {
            let payload = snap_proto::event::LoRaGatewayStatus {
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
            };

            let _ = self.tx.send(Ok(LogResponse {
                identity: Some(DeviceIdentity { id: gate.id.0, eui: gate.eui.0 }),
                log_type: LogType::LoRaGatewayStatus as _,
                payload: payload.encode_length_delimited_to_vec(),
            }));
        }
    }
    pub(crate) async fn lora_node_join_request(
        &self,
        _data: &PushData,
        device: &NodeInfo,
    ) {

        let payload = snap_proto::event::LoRaNodeJoinRequest {
            app_eui: device.app_eui.0,
            dev_eui: device.dev_eui.0,
            time: chrono::Utc::now().timestamp_millis(),
        };
        let _ = self.tx.send(Ok(LogResponse {
            identity: Some(DeviceIdentity { id: device.device_id.0, eui: device.dev_eui.0 }),
            log_type: LogType::LoRaNodeJoinRequest as _,
            payload: payload.encode_length_delimited_to_vec(),
        }));
    }

    pub(crate) async fn lora_node_join_accept(
        &self,
        addr: LoRaAddr,
        device: &NodeInfo,
    ) {
        let payload = snap_proto::event::LoRaNodeJoinAccept {
            dev_addr: addr.0,
            time: chrono::Utc::now().timestamp_millis(),
        };
        let _ = self.tx.send(Ok(LogResponse {
            identity: Some(DeviceIdentity { id: device.device_id.0, eui: device.dev_eui.0 }),
            log_type: LogType::LoRaNodeJoinAccept as _,
            payload: payload.encode_length_delimited_to_vec(),
        }));
    }

    pub(crate) async fn lora_node_uplink_data(
        &self,
        header: &LoRaPayload,
        device: &LoRaNode,
        gateway: &PushData,
        data: &[u8],
    ) {
        let gateway = snap_proto::event::GatewayRxStatus {
            id: gateway.gateway.0,
            eui: gateway.eui.0,
            time: gateway.time.timestamp_millis() as i64,
            rssi: gateway.pk.rssi,
            snr: gateway.pk.lsnr,
        };
        let payload = snap_proto::event::LoRaNodeUplinkData {
            dev_addr: device.info.dev_addr.0,
            confirm: header.is_confirmed(),
            f_port: header.f_port().unwrap_or_default() as _,
            f_cnt: header.fhdr().fcnt() as _,
            payload: Some(header.as_bytes().encode_base64()),
            decoded_payload: Some(data.encode_base64()),
            gateway: Some(gateway),
            time: chrono::Utc::now().timestamp_millis(),
        };
        let _ = self.tx.send(Ok(LogResponse {
            identity: Some(DeviceIdentity { id: device.info.device_id.0, eui: device.info.dev_eui.0 }),
            log_type: LogType::LoRaNodeUplinkData as _,
            payload: payload.encode_length_delimited_to_vec(),
        }));

    }

    pub(crate) async fn lora_node_downlink_data(
        &self,
        _data: &PushData,
        device: &NodeInfo,
        down: Option<&DownloadData>,
    ) {
        let payload = snap_proto::event::LoRaNodeDownLinkData {
            confirm: false,
            f_port: down.map(|i| i.port).unwrap_or(2) as i32,
            bytes: down.map(|i| {
                base64::engine::general_purpose::STANDARD.encode(i.bytes.as_ref())
            }),
            time: chrono::Utc::now().timestamp_millis(),
        };
        let _ = self.tx.send(Ok(LogResponse {
            identity: Some(DeviceIdentity { id: device.device_id.0, eui: device.dev_eui.0 }),
            log_type: LogType::LoRaNodeJoinAccept as _,
            payload: payload.encode_length_delimited_to_vec(),
        }));
    }
}
