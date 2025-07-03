use crate::man::data::DownloadData;
use crate::man::lora::LoRaNodeManager;
use crate::man::mqtt::MqPublisher;
use crate::man::redis_client::{RedisClient, RedisRecv};
use crate::protocol::snap::DownJson;
use crate::{DeviceError, DeviceResult, GLOBAL_STATE};
use base64::Engine;
use common_define::event::DownloadMessage;
use common_define::ClientId;
use device_info::snap::SnapDeviceInfo;
use redis::Msg;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tracing::{info, warn};

pub struct DownlinkManager {
    recv: mpsc::Receiver<DownloadMessage>,
}
impl DownlinkManager {
    pub fn new(recv: mpsc::Receiver<DownloadMessage>) -> DownlinkManager {
        DownlinkManager { recv }
    }

    pub async fn start_downlink(&mut self) {
        loop {
            while let Some(msg) = self.recv.recv().await {
                match Self::process_downlink(msg).await {
                    Ok(_) => {}
                    Err(e) => {
                        warn!("Error processing downlink: {}", e);
                    }
                }
            }
        }
    }

    async fn process_downlink(down: DownloadMessage) -> DeviceResult {
        if let Some(s) = LoRaNodeManager::get_node_by_eui(down.eui).await? {
            let data =
                base64::engine::general_purpose::STANDARD.decode(down.data.as_bytes())?;
            info!("{}, down message", down.eui);
            tokio::spawn(async move {
                if let Err(e) = s
                    .dispatch_task_now(DownloadData::new_data_with_id(
                        data,
                        1,
                        down.port,
                    ))
                    .await
                {
                    info!(device = down.eui.to_string(), "forward error: {}", e);
                }
            });
            return Ok(());
        }
        let mut conn = RedisClient::get_client().get_multiplexed_conn().await?;

        if let Some(snap) = SnapDeviceInfo::load(down.eui, &mut conn).await? {
            if let Some(down_link) = snap.down {
                if snap.freq.is_none() {
                    return Err(DeviceError::Device("snap device not found freq".to_string()));
                }
                match crate::protocol::snap::DownloadData::new_with_eui(down.eui)
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
                        let s = serde_json::to_string(&j)?;
                        // if let Err(e) = GLOBAL_STATE.mq.publish(down_link, s).await {
                        //     warn!("failed to publish down: {}", e);
                        // }
                    }
                    Err(e) => {
                        warn!("failed to encode ack: {:?}", e);
                    }
                }
            }
        }
        Ok(())
    }
}
