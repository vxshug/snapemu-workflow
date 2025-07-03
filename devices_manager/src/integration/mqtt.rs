use derive_new::new;
use tracing::info;
use crate::man::data::ValueType;
use crate::man::Id;
use crate::DeviceResult;
use common_define::db::{Eui, LoRaAddr};
use common_define::decode::Value;
use crate::decode::DecodeData;
use crate::man::lora::LoRaNode;

#[derive(new)]
pub(crate) struct MqttMessage {
    pub(crate) message: String,
    pub(crate) topic: String,
}


impl MqttMessage {
    pub(crate) fn new_one_data(data: &MqttData,) -> DeviceResult<Self> {
        let message = serde_json::to_string(data)?;
        let topic = format!("/v1/device/{}/data/{}", data.device.unwrap(), data.data_id);
        Ok(Self { message, topic })
    }

    pub(crate) fn new_data(data: &MqttDataAll) -> DeviceResult<Self> {
        let message = serde_json::to_string(data)?;
        let topic = format!("/v1/device/{}/data", data.device);
        Ok(Self { message, topic })
    }

    pub(crate) fn new_row_data(data: &MqttRawData, qos: i32) -> DeviceResult<Self> {
        let message = serde_json::to_string(data)?;
        let topic = format!("/v1/device/{}/row", data.device);
        Ok(Self { message, topic })
    }

    pub(crate) fn new_decode_data(data: &DecodeData, node: &LoRaNode) -> Option<Self> {
        if let Some(user_id) = node.info.user_id {
            let topic = format!("user/{}/data/{}/uplink", user_id, node.info.dev_eui);
            let message = MqttDecodeData {
                battery: node.info.battery,
                charge: Some(node.info.charge),
                eui: node.info.dev_eui,
                data: data.data.iter().map(|item| MqttDataItem {
                    data: item.v.clone(),
                    id: item.i,
                    name: item.name.clone(),
                    unit: item.unit.clone(),
                } ).collect(),
            };
            let message = serde_json::to_string(&message).ok()?;
            return Some(Self { message, topic })
        };
        None
    }

    pub(crate) fn new_decode_group_data(
        data: &MqttDecodeData,
        group_id: Id,
    ) -> DeviceResult<Self> {
        let message = serde_json::to_string(data)?;
        let topic = format!("/v1/group/{}/decode", group_id);
        Ok(Self { message, topic })
    }
}

#[derive(serde::Serialize)]
pub(crate) struct MqttData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) device: Option<Id>,
    pub(crate) data_id: i32,
    pub(crate) s_id: i32,
    pub(crate) pk_id: i16,
    pub(crate) v_type: ValueType,
    pub(crate) data: Value,
    pub(crate) bytes: String,
}

#[derive(serde::Serialize)]
pub(crate) struct MqttDataAll {
    pub(crate) device: Id,
    pub(crate) data: Vec<MqttData>,
}

#[derive(serde::Serialize)]
pub(crate) struct MqttRawData {
    pub(crate) device: Id,
    pub(crate) bytes: String,
}

#[derive(serde::Serialize)]
pub(crate) struct MqttDecodeData {
    pub(crate) battery: Option<i16>,
    pub(crate) charge: Option<bool>,
    pub(crate) eui: Eui,
    pub(crate) data: Vec<MqttDataItem>,
}

#[derive(serde::Serialize)]
pub(crate) struct MqttDataItem {
    pub(crate) data: Value,
    pub(crate) id: i32,
    pub(crate) name: String,
    pub(crate) unit: String,
}
