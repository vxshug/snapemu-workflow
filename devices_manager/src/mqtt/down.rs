use crate::man::mqtt::MqttMessage;
use crate::mqtt::down::message::{ForwardStatus, RawData};
use crate::DeviceResult;
use base64::Engine;
use common_define::db::Eui;
use derive_new::new;
use tokio::sync::mpsc;
use tracing::warn;

pub mod message {
    use common_define::db::Eui;
    use derive_new::new;
    use serde::Serialize;

    #[derive(Serialize)]
    pub enum StatusResult {
        Ok,
        Auth,
        Error,
    }

    #[derive(Serialize, new)]
    pub struct ForwardStatus<'a> {
        pub id: u32,
        pub eui: Eui,
        pub result: &'a str,
    }

    #[derive(Serialize, new)]
    pub struct RawData<'a> {
        pub port: u8,
        pub data: &'a str,
    }
}

#[derive(Clone, new)]
pub struct MqttDownload {
    conn: mpsc::Sender<MqttMessage>,
}

pub struct RedisMessageBirge {}

impl MqttDownload {
    pub async fn forward_status(
        &mut self,
        eui: Eui,
        message_id: u32,
        status: &[u8],
    ) -> DeviceResult {
        let p = base64::engine::general_purpose::STANDARD.encode(status);
        let status = ForwardStatus::new(message_id, eui, &p);
        let status = serde_json::to_vec(&status)?.into();
        let _ = self
            .conn
            .send(MqttMessage::new(format!("device/{}/forward_status", eui), status))
            .await
            .map_err(|e| {
                warn!("Failed to send forward status to Mqtt: {}", e);
            });
        Ok(())
    }

    pub async fn raw(&mut self, eui: Eui, port: u8, raw: &[u8]) -> DeviceResult {
        let p = base64::engine::general_purpose::STANDARD.encode(raw);
        let data = RawData::new(port, &p);
        let data = serde_json::to_vec(&data)?.into();
        let _ = self
            .conn
            .send(MqttMessage::new(format!("device/{}/forward_status", eui), data))
            .await
            .map_err(|e| {
                warn!("Failed to send forward status to Mqtt: {}", e);
            });
        Ok(())
    }
}
