use crate::decode::{up_data_decode, RawData};
use crate::man::mqtt::{MqttMessage, MqPublisher};
use crate::man::redis_client::RedisClient;
use crate::protocol::snap::{DownJson, DownloadData, UpData, UpJson};
use crate::{DeviceError, DeviceResult, GLOBAL_DEPEND, GLOBAL_STATE};
use base64::Engine;
use common_define::db::{
    DbDecodeData, DeviceDataActiveModel, DevicesEntity, SnapDeviceColumn, SnapDeviceEntity,
};
use common_define::decode::LastDecodeData;
use common_define::last_device_data_key;
use common_define::time::Timestamp;
use derive_new::new;
use device_info::snap::SnapDeviceInfo;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use common_define::event::DownloadMessage;
use utils::base64::EncodeBase64;

pub async fn start_process_snap(mut rx: mpsc::Receiver<MqttMessage>) {
    while let Some(message) = rx.recv().await {
        debug!("start process topic: {}", message.topic.as_str());

        let result = serde_json::from_slice::<UpJson>(message.payload.as_ref());
        if let Ok(ok) = result {
            tokio::spawn(decode_custom_gateway(ok, message.topic));
        }
    }
}

pub async fn decode_custom_gateway(up: UpJson, topic: String) {
    let redis_conn = match RedisClient::get_client().get_multiplexed_conn().await {
        Ok(redis_conn) => redis_conn,
        Err(e) => {
            error!("get redis is error: {}", e);
            return;
        }
    };

    NodeProcessor::new(topic, up, redis_conn).start().await;
}

#[derive(new)]
struct NodeProcessor {
    topic: String,
    pk: UpJson,
    redis: MultiplexedConnection,
}

impl NodeProcessor {
    async fn start(mut self) {
        if let Err(e) = self.wrap().await {
            warn!("topic: {}, error: {}", self.topic, e);
        }
    }

    async fn wrap(&mut self) -> DeviceResult {
        let bytes = base64::engine::general_purpose::STANDARD.decode(&self.pk.data)?;
        let mut up = UpData::new(bytes)
            .map_err(|e| DeviceError::Device(format!("invalid bytes error: {:?}", e)))?;
        let eui = up.eui();
        info!("device eui is: {}", eui);
        let snap_device = match SnapDeviceInfo::load(eui, &mut self.redis).await? {
            None => {
                let (snap, device) = SnapDeviceEntity::find()
                    .filter(SnapDeviceColumn::Eui.eq(eui))
                    .find_also_related(DevicesEntity)
                    .one(&GLOBAL_STATE.db)
                    .await?
                    .ok_or_else(|| {
                        warn!("not found devices");
                        DeviceError::Device("error".to_string())
                    })?;
                let device = device
                    .ok_or_else(|| DeviceError::device(format!("device not found: {}", eui)))?;
                let down = self.topic.replace("up", "down");

                let info = SnapDeviceInfo::new(
                    snap.device_id,
                    snap.key,
                    Some(Timestamp::now()),
                    1,
                    Some(down),
                    device.script,
                    Some(self.pk.freq),
                );
                info.register(snap.eui, &mut self.redis).await?;
                info
            }
            Some(d) => d,
        };
        let ack = up.ack();
        let counter = up.counter();

        let payload = up
            .decode_payload(&snap_device.key)
            .map_err(|_| DeviceError::Device(format!("eui: {} device key is not match", eui)))?;
        let resp = common_define::event::DeviceEvent {
            device: snap_device.id,
            event: common_define::event::DeviceEventType::SnapDevice(
                common_define::event::SnapEvent { eui, data: payload.to_vec() },
            ),
        };
        let resp = serde_json::to_string(&resp)?;
        let now = Timestamp::now();
        SnapDeviceInfo::update_active_time(eui, now, &mut self.redis).await?;
        self.redis.publish(common_define::event::DeviceEvent::KAFKA_TOPIC, resp).await?;

        if ack {
            match DownloadData::new_with_eui(eui)
                .set_ack()
                .set_counter(counter)
                .encode_payload(&[], &snap_device.key)
            {
                Ok(o) => {
                    let j = DownJson { token: rand::random(), freq: self.pk.freq, data: o };
                    let s = serde_json::to_string(&j)?;
                    let down = self.topic.replace("up", "down");
                    if snap_device.down.as_ref() != Some(&down) {
                        SnapDeviceInfo::update_down(eui, down.clone(), &mut self.redis).await?;
                    }
                    if snap_device.freq.as_ref() != Some(&self.pk.freq) {
                        SnapDeviceInfo::update_freq(eui, self.pk.freq, &mut self.redis).await?;
                    }
                    // if let Err(e) = MqPublisher::publish(down, s).await {
                    //     warn!("failed to publish down: {}", e);
                    // }
                }
                Err(e) => {
                    warn!("failed to encode ack: {:?}", e);
                }
            }
        }
        match snap_device.script {
            Some(o) => {
                let script = common_define::db::DecodeScriptEntity::find_by_id(o)
                    .one(&GLOBAL_STATE.db)
                    .await?;
                match script {
                    None => {
                        warn!("Not found Script");
                    }
                    Some(script) => {
                        let bytes_b64 = payload.encode_base64();
                        let decodedata = GLOBAL_DEPEND
                            .decode_with_code(script.script.as_str(), RawData::new(payload))
                            .map_err(|e| DeviceError::data("js decode"))?;
                        if decodedata.data.is_empty() {
                            warn!("js return null");
                            return Ok(());
                        }
                        let last_key = last_device_data_key(snap_device.id);
                        let data: DbDecodeData = decodedata.into();
                        let now = Timestamp::now();
                        let last_data = LastDecodeData::new(data.0.clone(), now.timestamp_millis() as _);
                        self.redis.set(last_key, last_data).await?;
                        let data = DeviceDataActiveModel {
                            id: Default::default(),
                            device_id: ActiveValue::Set(snap_device.id),
                            data: ActiveValue::Set(data),
                            bytes: ActiveValue::Set(bytes_b64),
                            create_time: ActiveValue::Set(now),
                        };
                        data.insert(&GLOBAL_STATE.db).await?;
                    }
                }
            }
            None => {
                let decoded_data = up_data_decode(payload)?;
                let last_data = LastDecodeData::new(decoded_data.data.clone(), now.timestamp_millis() as _);
                info!("decode {:?}", decoded_data);
                let last_key = last_device_data_key(snap_device.id);
                self.redis.set(last_key, last_data).await?;
                let bytes_b64 = payload.encode_base64();
                let data = DeviceDataActiveModel {
                    id: Default::default(),
                    device_id: ActiveValue::Set(snap_device.id),
                    data: ActiveValue::Set(DbDecodeData(decoded_data.data)),
                    bytes: ActiveValue::Set(bytes_b64),
                    create_time: ActiveValue::Set(now),
                };
                data.insert(&GLOBAL_STATE.db).await?;
            }
        }

        Ok(())
    }
}
