use common_define::db::{
    DecodeScriptColumn, DecodeScriptEntity, DeviceAuthorityActiveModel, DeviceAuthorityColumn,
    DeviceAuthorityEntity, DeviceAuthorityModel, DeviceDataEntity, DeviceDataModel,
    DeviceFunctionColumn, DeviceFunctionEntity, DeviceFunctionModel, DeviceLoraGateActiveModel,
    DeviceLoraGateColumn, DeviceLoraGateEntity, DeviceLoraGateModel, DeviceLoraNodeColumn,
    DeviceLoraNodeEntity, DeviceLoraNodeModel, DevicesActiveModel, DevicesColumn, DevicesEntity,
    DevicesModel, Eui, Key, LoRaAddr, SnapDeviceColumn, SnapDeviceDataNameColumn,
    SnapDeviceDataNameEntity, SnapDeviceEntity, SnapDeviceModel,
};
use common_define::decode::LastDecodeData;
use common_define::lora::{LoRaJoinType, LoRaRegion};
use common_define::product::{DeviceType, ProductType, ShareType};
use common_define::time::Timestamp;
use common_define::{last_device_data_key, Id};
use derive_new::new;
use device_info::lorawan::NodeInfo;
use redis::AsyncCommands;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel,
    ModelTrait, QueryFilter, Statement,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, instrument};

use super::DeviceService;
use crate::cache::DeviceCache;
use crate::error::ApiError;
use crate::error::ApiResult;
use crate::gw_conf::GwConfig;
use crate::service::data::query::{DataDeviceOneResponse, TimeDate};
use crate::service::data::DataService;
use crate::service::device::group::{DeviceGroupResp, DeviceGroupService};
use crate::service::lorawan::{LoRaGateService, LoRaNodeService, ReqLoraGateway, ReqLoraNode};
use crate::service::mqtt::{MQTTService, ReqMQTT};
use crate::service::snap::{ReqSnap, SnapDeviceService, SnapJoinParameter};
use crate::{
    get_lang, tt, AppState, CurrentUser, DEVICE_DATA_RAW_SQL, MODEL_MAP, SEA_ORMDB_BACKEND,
};

#[derive(Serialize)]
pub(crate) struct DeviceIoResp {
    pub pin: i16,
    pub modify: bool,
    pub output: bool,
    pub value: bool,
    pub update_time: Timestamp,
}

#[derive(Serialize)]
pub(crate) struct DeviceTimerResp {
    pub num: i16,
    pub enable: bool,
    pub pin: i16,
    pub action: bool,
    pub hour: i16,
    pub minute: i16,
    pub repeat: i16,
}

#[derive(Serialize)]
pub(crate) struct DeviceIoWrapResp {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) io: Option<Vec<DeviceIoResp>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timer: Option<Vec<DeviceTimerResp>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct DeviceSource {
    pub share_type: ShareType,
    pub owner: bool,
    pub manager: bool,
    pub modify: bool,
    pub delete: bool,
    pub share: bool,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct DeviceResp {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) id: Option<Id>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) eui: Option<Eui>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) blue_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) online: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) period: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) battery: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<Vec<DataDeviceOneResponse>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) charge: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) info: Option<DeviceInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source: Option<DeviceSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) device_type: Option<DeviceType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) product_type: Option<ProductType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) create_time: Option<Timestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) active_time: Option<Timestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) script: Option<Id>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) product_id: Option<Id>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) product_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) group: Option<Vec<DeviceGroupResp>>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub(crate) enum DeviceInfo {
    MQTT(MQTTDeviceInfo),
    LoRaNode(LoRaNodeDeviceInfo),
    LoRaGate(LoRaGateDeviceInfo),
    Snap(SnapDeviceInfo),
}

impl From<SnapDeviceModel> for SnapDeviceInfo {
    fn from(value: SnapDeviceModel) -> Self {
        Self {
            device_id: value.device_id,
            eui: value.eui,
            key: value.key,
            battery: value.battery,
            charge: value.charge.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, new)]
pub(crate) struct SnapDeviceInfo {
    pub(crate) device_id: Id,
    pub(crate) eui: Eui,
    pub(crate) key: Key,
    pub(crate) battery: Option<i16>,
    pub(crate) charge: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, new)]
pub(crate) struct MQTTDeviceInfo {
    pub(crate) device_id: Id,
    pub(crate) eui: Eui,
    pub(crate) username: String,
    pub(crate) password: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct LoRaNodeDeviceInfo {
    pub(crate) region: LoRaRegion,
    pub(crate) join_type: LoRaJoinType,
    pub(crate) app_eui: Eui,
    pub(crate) dev_eui: Eui,
    pub(crate) app_key: Key,
    pub(crate) dev_addr: LoRaAddr,
    pub(crate) nwk_skey: Key,
    pub(crate) app_skey: Key,
    pub(crate) class_b: bool,
    pub(crate) class_c: bool,
    pub(crate) adr: bool,
    pub(crate) rx1_delay: i16,
    pub(crate) des_rx1_delay: i16,

    pub(crate) rx1_dro: i16,
    pub(crate) des_rx1_dro: i16,

    pub(crate) rx2_dr: i16,
    pub(crate) des_rx2_dr: i16,

    pub(crate) rx2_freq: i32,
    pub(crate) des_rx2_freq: i32,

    pub(crate) d_retry: i16,
    pub(crate) c_retry: i16,
    pub(crate) dutycyle: i32,
    pub(crate) product_type: ProductType,

    pub(crate) up_confirm: Option<bool>,
    pub(crate) up_dr: Option<i16>,
    pub(crate) power: Option<i16>,
    pub(crate) battery: Option<i16>,
    pub(crate) charge: Option<bool>,
    pub(crate) time_zone: Option<i32>,
    pub(crate) firmware: Option<i32>,
    pub(crate) dev_non: Option<i32>,
    pub(crate) app_non: Option<i32>,
    pub(crate) net_id: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DeviceModify {
    pub name: Option<String>,
    pub description: Option<String>,
    pub period: Option<i32>,
    pub script: Option<Id>,
    pub reset_script: Option<bool>,
    pub region: Option<LoRaRegion>,
    pub join_type: Option<LoRaJoinType>,
    pub app_eui: Option<Eui>,
    pub app_key: Option<Key>,
    pub nwk_skey: Option<Key>,
    pub app_skey: Option<Key>,
    pub class_c: Option<bool>,
    pub product_id: Option<Id>,
}

impl From<DeviceLoraNodeModel> for LoRaNodeDeviceInfo {
    fn from(value: DeviceLoraNodeModel) -> Self {
        Self {
            region: value.region,
            join_type: value.join_type,
            app_eui: value.app_eui,
            dev_eui: value.dev_eui,
            app_key: value.app_key,
            dev_addr: value.dev_addr,
            nwk_skey: value.nwk_skey,
            app_skey: value.app_skey,
            class_b: value.class_b,
            class_c: value.class_c,
            adr: value.adr,
            rx1_delay: value.rx1_delay,
            des_rx1_delay: value.des_rx1_delay,
            rx1_dro: value.rx1_dro,
            des_rx1_dro: value.des_rx1_dro,
            rx2_dr: value.rx2_dr,
            des_rx2_dr: value.des_rx2_dr,
            rx2_freq: value.rx2_freq,
            des_rx2_freq: value.des_rx2_freq,
            d_retry: value.d_retry,
            c_retry: value.c_retry,
            dutycyle: value.dutycyle,
            product_type: value.product_type,
            up_confirm: value.up_confirm.into(),
            up_dr: value.up_dr.into(),
            power: value.power.into(),
            battery: value.battery,
            charge: value.charge.into(),
            time_zone: value.time_zone.into(),
            firmware: value.firmware.into(),
            dev_non: value.dev_non.into(),
            app_non: value.app_non.into(),
            net_id: value.net_id.into(),
        }
    }
}
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct LoRaGateDeviceInfo {
    pub(crate) device_id: Id,
    pub(crate) region: LoRaRegion,
    pub(crate) eui: Eui,
}
impl From<DeviceLoraGateModel> for LoRaGateDeviceInfo {
    fn from(value: DeviceLoraGateModel) -> Self {
        Self { device_id: value.device_id, region: value.region, eui: value.eui }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct DevicePredefineResp {
    name: String,
    msg: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct DeviceCreate {
    pub(crate) name: String,
    pub(crate) group: Vec<Id>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) device: Option<Id>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) eui: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) scan_eui: Option<String>,
    pub(crate) device_type: DeviceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) region: Option<LoRaRegion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) join_type: Option<LoRaJoinType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) join_parameter: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) blue_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) blue_parm: Option<BluetoothNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) extra_parm: Option<ExtraParm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) password: Option<String>,
}
#[derive(Deserialize, Debug)]
pub(crate) struct ExtraParm {
    pub(crate) class_b: Option<bool>,
    pub(crate) class_c: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct BluetoothNode {
    pub(crate) product: String,
    pub(crate) region: LoRaRegion,
    pub(crate) join_type: Option<i32>,
    pub(crate) jion_type: Option<i32>,
    pub(crate) class: String,
    #[serde(rename = "ADR")]
    pub(crate) adr: i32,
    #[serde(rename = "DR")]
    pub(crate) dr: i32,
    #[serde(rename = "Confirmed")]
    pub(crate) confirmed: i32,
    #[serde(rename = "DutyCycle")]
    pub(crate) duty_cycle: i32,
    #[serde(rename = "Power")]
    pub(crate) power: i32,
    #[serde(rename = "Retry")]
    pub(crate) retry: i32,
    #[serde(rename = "Channel")]
    pub(crate) channel: Option<String>,
    pub(crate) dev_eui: Eui,
    pub(crate) app_eui: Eui,
    pub(crate) app_key: Key,
    pub(crate) dev_addr: LoRaAddr,
    pub(crate) nwk_skey: Key,
    pub(crate) app_skey: Key,
    #[serde(rename = "RX1DELAY")]
    pub(crate) rx1_delay: i32,
    #[serde(rename = "RX2DELAY")]
    pub(crate) rx2_delay: i32,
    #[serde(rename = "RX2DR_TYPE")]
    pub(crate) rx2_dr_type: i32,
    #[serde(rename = "RX2FREQ_TYPE")]
    pub(crate) rx2_freq_type: i32,
    #[serde(rename = "RX2DR")]
    pub(crate) rx2_dr: i32,
    #[serde(rename = "RX2FREQ")]
    pub(crate) rx2_freq: i32,
    #[serde(rename = "Firmware")]
    pub(crate) firmware: Option<String>,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct DeviceWithAuth {
    pub auth: DeviceAuthorityModel,
    pub device: DevicesModel,
}
impl DeviceService {
    pub(crate) async fn delete_by_user_id<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        user_id: Id,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult {
        let mut can_delete = vec![];
        let mut lora_node = vec![];
        let mut lora_gate = vec![];
        let mut mqtt = vec![];
        let mut snap = vec![];
        let devices = Self::query_all_with_auth(user_id, redis, conn).await?;
        for device in devices.v {
            if device.auth.owner {
                can_delete.push(device.device.id);
                match device.device.device_type {
                    DeviceType::LoRaNode => lora_node.push(device.device.id),
                    DeviceType::LoRaGate => lora_gate.push(device.device.id),
                    DeviceType::MQTT => mqtt.push(device.device.id),
                    DeviceType::Snap => snap.push(device.device.id),
                }
            }
        }

        // delete snap device
        Self::delete_snap(lora_node.as_slice(), conn).await?;

        // delete lora node
        Self::delete_lora_node(lora_node.as_slice(), conn).await?;

        // delete lora gateway
        Self::delete_lora_gateway(lora_gate.as_slice(), conn).await?;

        // delete data
        DataService::delete_by_device_id_array(can_delete.as_slice(), conn).await?;

        // delete device
        Self::delete_list(can_delete.as_slice(), conn).await?;
        Self::delete_device_auth(can_delete.as_slice(), conn).await?;
        DeviceCache::delete_by_user_id(user_id, redis).await?;

        Ok(())
    }

    pub(crate) async fn delete_snap<C: ConnectionTrait>(device_id: &[Id], conn: &C) -> ApiResult {
        SnapDeviceEntity::delete_many()
            .filter(SnapDeviceColumn::DeviceId.is_in(device_id))
            .exec(conn)
            .await?;
        Ok(())
    }

    pub(crate) async fn delete_lora_node<C: ConnectionTrait>(
        device_id: &[Id],
        conn: &C,
    ) -> ApiResult {
        DeviceLoraNodeEntity::delete_many()
            .filter(DeviceLoraNodeColumn::DeviceId.is_in(device_id))
            .exec(conn)
            .await?;
        Ok(())
    }

    pub(crate) async fn delete_lora_gateway<C: ConnectionTrait>(
        device_id: &[Id],
        conn: &C,
    ) -> ApiResult {
        DeviceLoraGateEntity::delete_many()
            .filter(DeviceLoraGateColumn::DeviceId.is_in(device_id))
            .exec(conn)
            .await?;
        Ok(())
    }

    pub(crate) async fn delete_device_auth<C: ConnectionTrait>(
        device_id: &[Id],
        conn: &C,
    ) -> ApiResult {
        DeviceAuthorityEntity::delete_many()
            .filter(DeviceAuthorityColumn::DeviceId.is_in(device_id))
            .exec(conn)
            .await?;
        Ok(())
    }

    pub(crate) async fn delete_list<C: ConnectionTrait>(device_id: &[Id], conn: &C) -> ApiResult {
        DevicesEntity::delete_many().filter(DevicesColumn::Id.is_in(device_id)).exec(conn).await?;
        Ok(())
    }
    pub(crate) async fn delete<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        device_id: Id,
        user: &CurrentUser,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult {
        let DeviceWithAuth { auth, device } = Self::query_one_with_auth(user.id, device_id, conn)
            .await
            .map_err(|_| ApiError::User("invalid device".into()))?;
        if auth.owner {
            DeviceGroupService::unlink_device_all(device.id, redis, conn).await?;
            match device.device_type {
                DeviceType::Snap => {
                    SnapDeviceService::delete(device_id, redis, conn).await?;
                    DataService::delete_by_device_id(device.id, conn).await?;
                }
                DeviceType::MQTT => {
                    MQTTService::delete(device_id, conn).await?;
                }
                DeviceType::LoRaNode => {
                    LoRaNodeService::delete_node(device.id, redis, conn).await?;
                    DataService::delete_by_device_id(device.id, conn).await?;
                }
                DeviceType::LoRaGate => {
                    LoRaGateService::delete_gateway(device.id, redis, conn).await?;
                }
            }
            device.delete(conn).await?;
            DeviceAuthorityEntity::delete_many()
                .filter(DeviceAuthorityColumn::DeviceId.eq(device_id))
                .exec(conn)
                .await?;
            return Ok(());
        }
        Err(ApiError::User("not owner device".into()))
    }

    pub(crate) async fn valid_eui<C: ConnectionTrait>(eui: Eui, conn: &C) -> ApiResult {
        let res = DevicesEntity::find().filter(DevicesColumn::Eui.eq(eui)).one(conn).await?;

        if res.is_some() {
            return Err(ApiError::User(tt!("messages.device.lora.dev_eui_already", eui = eui)));
        }
        Ok(())
    }

    #[instrument(skip_all)]
    pub(crate) async fn new_device<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        user: &CurrentUser,
        req: DeviceCreate,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult<Id> {
        let eui: Eui = req
            .eui
            .ok_or(ApiError::User(tt!("messages.device.common.eui_missing")))?
            .as_str()
            .parse()
            .map_err(|_| ApiError::User(tt!("messages.device.common.eui_format")))?;
        let device_id = match req.device_type {
            DeviceType::Snap => {
                let join_parameter = req
                    .join_parameter
                    .ok_or(ApiError::User(tt!("messages.device.lora.join_parameter_missing")))?;
                let join_parameter: SnapJoinParameter = serde_json::from_value(join_parameter)?;
                let req = ReqSnap::new(
                    req.name,
                    req.description.unwrap_or_default(),
                    eui,
                    join_parameter,
                );
                SnapDeviceService::create(req, user, redis, conn).await?.device_id
            }
            DeviceType::MQTT => {
                let req_mqtt = ReqMQTT {
                    name: req.name,
                    description: req.description.unwrap_or_default(),
                    eui,
                    username: req
                        .username
                        .ok_or(ApiError::User(tt!("messages.device.mqtt.username_missing")))?,
                    password: req
                        .password
                        .ok_or(ApiError::User(tt!("messages.device.mqtt.password_missing")))?,
                };
                MQTTService::create(req_mqtt, user, conn).await?.device_id
            }
            DeviceType::LoRaGate => {
                let req_g = ReqLoraGateway {
                    name: req.name.clone(),
                    description: req.description.unwrap_or(req.name),
                    eui,
                    region: req
                        .region
                        .ok_or(ApiError::User(tt!("messages.device.common.region_missing")))?,
                };
                LoRaGateService::create(req_g, user, redis, conn).await?.device_id
            }
            DeviceType::LoRaNode => {
                let req_g = ReqLoraNode {
                    eui,
                    device: req.device,
                    name: req.name.clone(),
                    description: req.description.unwrap_or(req.name),
                    region: req
                        .region
                        .ok_or(ApiError::User(tt!("messages.device.common.region_missing")))?,
                    join_type: req
                        .join_type
                        .ok_or(ApiError::User(tt!("messages.device.lora.join_type_missing")))?,
                    join_parameter: serde_json::from_value(req.join_parameter.ok_or(
                        ApiError::User(tt!("messages.device.lora.join_parameter_missing")),
                    )?)?,
                    scan_eui: req.scan_eui,
                    blue_name: req.blue_name,
                    blue_parm: req.blue_parm,
                    extra_parm: req.extra_parm,
                };
                LoRaNodeService::create_node(req_g, user, redis, conn).await?
            }
        };
        let default_group =
            DeviceGroupService::link_to_default_group(user.id, device_id, redis, conn).await?;

        for group in req.group {
            if default_group.group_id == default_group.id {
                continue;
            }
            DeviceGroupService::link(group, user.id, device_id, redis, conn).await?;
        }
        let auth = DeviceAuthorityActiveModel {
            id: Default::default(),
            auth_creator: ActiveValue::Set(user.id),
            device_id: ActiveValue::Set(device_id),
            share_type: ActiveValue::Set(ShareType::User),
            share_id: ActiveValue::Set(user.id),
            owner: ActiveValue::Set(true),
            manager: ActiveValue::Set(true),
            modify: ActiveValue::Set(true),
            delete: ActiveValue::Set(true),
            share: ActiveValue::Set(true),
            create_time: ActiveValue::Set(Timestamp::now()),
        };
        auth.insert(conn).await?;
        Ok(device_id)
    }

    pub(crate) async fn register_device<C: ConnectionTrait>(
        user: &CurrentUser,
        eui: Eui,
        device_name: &str,
        desc: &str,
        device_type: DeviceType,
        conn: &C,
    ) -> ApiResult<DevicesModel> {
        let device = DevicesActiveModel {
            id: Default::default(),
            eui: ActiveValue::Set(eui),
            name: ActiveValue::Set(device_name.to_string()),
            description: ActiveValue::Set(desc.to_string()),
            creator: ActiveValue::Set(user.id),
            enable: ActiveValue::Set(true),
            period: ActiveValue::Set(60),
            script: ActiveValue::Set(None),
            data_id: Default::default(),
            product_id: Default::default(),
            device_type: ActiveValue::Set(device_type),
            active_time: ActiveValue::Set(None),
            create_time: ActiveValue::Set(Timestamp::now()),
        };
        let device = device.insert(conn).await?;
        Ok(device)
    }

    #[instrument(skip(conn, redis))]
    pub(crate) async fn query_all<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        user_id: Id,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult<Vec<DevicesModel>> {
        Self::query_all_with_auth(user_id, redis, conn)
            .await
            .map(|item| item.v.into_iter().map(|it| it.device).collect())
    }
    pub(crate) async fn query_all_with_auth<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        user_id: Id,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult<DeviceCache> {
        let devices = DeviceAuthorityEntity::find()
            .filter(
                DeviceAuthorityColumn::ShareId
                    .eq(user_id)
                    .and(DeviceAuthorityColumn::ShareType.eq(ShareType::User.as_ref())),
            )
            .find_also_related(DevicesEntity)
            .all(conn)
            .await?;
        let devices = devices
            .into_iter()
            .filter_map(|item| match item.1 {
                Some(device) => Some(DeviceWithAuth { device, auth: item.0 }),
                None => None,
            })
            .collect::<Vec<_>>();
        let device_cache = DeviceCache::new(devices);
        Ok(device_cache)
    }
    pub(crate) async fn query_all_with_ids<C: ConnectionTrait>(
        user_id: Id,
        devices: &[Id],
        conn: &C,
    ) -> ApiResult<Vec<DevicesModel>> {
        let devices = DeviceAuthorityEntity::find()
            .filter(
                DeviceAuthorityColumn::ShareId
                    .eq(user_id)
                    .and(DeviceAuthorityColumn::ShareType.eq(ShareType::User.as_ref()))
                    .and(DeviceAuthorityColumn::DeviceId.is_in(devices)),
            )
            .find_also_related(DevicesEntity)
            .all(conn)
            .await?;
        let devices = devices.into_iter().flat_map(|item| item.1).collect::<Vec<_>>();
        Ok(devices)
    }
    pub(crate) async fn query_device_online_with_ids<C: ConnectionTrait>(
        devices: &[Id],
        conn: &C,
    ) -> ApiResult<HashMap<Id, u64>> {
        if devices.is_empty() {
            return Ok(HashMap::new());
        }
        let devices_ids = itertools::join(devices.iter().map(|i| i.0), ",");
        let device_data: Vec<DeviceDataModel> = DeviceDataEntity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                SEA_ORMDB_BACKEND,
                DEVICE_DATA_RAW_SQL.replace("parameter", &devices_ids),
                [],
            ))
            .all(conn)
            .await?;
        let mut h = HashMap::new();
        let mut predata: Option<(Id, u64)> = None;
        for data in device_data {
            if let Some((pre_id, pre_time)) = predata {
                let now_time = data.create_time.timestamp_millis();
                if pre_id == data.device_id {
                    h.insert(pre_id, pre_time - now_time);
                    continue;
                }
            }
            predata = Some((data.device_id, data.create_time.timestamp_millis()))
        }
        Ok(h)
    }
    #[instrument(skip(devices, state))]
    pub(crate) async fn query_last_data(
        devices: &[DevicesModel],
        state: &AppState,
    ) -> ApiResult<HashMap<Id, Vec<DataDeviceOneResponse>>> {
        if devices.is_empty() {
            return Ok(HashMap::new());
        }

        let mut conn = state.redis.get().await?;
        let mut map = HashMap::new();
        let ids = devices
            .iter()
            .filter(|it| {
                it.device_type == DeviceType::MQTT
                    || it.device_type == DeviceType::LoRaNode
                    || it.device_type == DeviceType::Snap
            })
            .map(|item| (item.id, last_device_data_key(item.id), item.script, item.data_id))
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return Ok(map);
        }
        let script_ids = ids.iter().filter_map(|id| id.2).collect::<Vec<_>>();
        let data_ids = ids.iter().filter_map(|id| id.3).collect::<Vec<_>>();
        let data_keys = ids.iter().map(|id| id.1.as_str()).collect::<Vec<_>>();
        let script_map = if script_ids.is_empty() {
            HashMap::new()
        } else {
            DecodeScriptEntity::find()
                .filter(DecodeScriptColumn::Id.is_in(script_ids))
                .all(&state.db)
                .await?
                .into_iter()
                .map(|it| (it.id, it))
                .collect()
        };
        let data_id_map = if data_ids.is_empty() {
            HashMap::new()
        } else {
            SnapDeviceDataNameEntity::find()
                .filter(SnapDeviceDataNameColumn::Id.is_in(data_ids))
                .all(&state.db)
                .await?
                .into_iter()
                .map(|it| (it.id, it))
                .collect()
        };
        let last_data: Vec<Option<LastDecodeData>> = conn.mget(data_keys).await?;
        let lang = get_lang().as_static_str();
        for ((device_id, _key, script, data_id), last_date) in ids.into_iter().zip(last_data) {
            let mut resp = vec![];
            if let Some(last_data) = last_date {
                match script {
                    Some(script_id) => {
                        if let Some(sc) = script_map.get(&script_id) {
                            let map = last_data
                                .v
                                .into_iter()
                                .map(|it| (it.i, it.v))
                                .collect::<HashMap<_, _>>();
                            for m in sc.map.iter() {
                                if let Some(value) = map.get(&m.id) {
                                    let data = DataDeviceOneResponse {
                                        name: m.name.to_string(),
                                        data_id: m.id,
                                        unit: m.unit.to_string(),
                                        data: TimeDate { time: last_data.t, data: value.clone() },
                                    };
                                    resp.push(data)
                                }
                            }
                        }
                    }
                    None => match data_id {
                        Some(data_id) => {
                            if let Some(sc) = data_id_map.get(&data_id) {
                                let map = last_data
                                    .v
                                    .into_iter()
                                    .map(|it| (it.i, it.v))
                                    .collect::<HashMap<_, _>>();
                                for m in sc.map.0.iter() {
                                    if let Some(value) = map.get(&m.id) {
                                        let data = DataDeviceOneResponse {
                                            name: m.name.to_string(),
                                            data_id: m.id,
                                            unit: m.unit.to_string(),
                                            data: TimeDate {
                                                time: last_data.t,
                                                data: value.clone(),
                                            },
                                        };
                                        resp.push(data)
                                    }
                                }
                            }
                        }
                        None => {
                            for d in last_data.v {
                                let data_name = MODEL_MAP.get_entry(d.i, lang);
                                let data = DataDeviceOneResponse {
                                    name: data_name.name.to_string(),
                                    data_id: d.i,
                                    unit: data_name.unit.to_string(),
                                    data: TimeDate { time: last_data.t, data: d.v },
                                };
                                resp.push(data)
                            }
                        }
                    },
                }
            }
            map.insert(device_id, resp);
        }
        Ok(map)
    }

    #[instrument(skip(conn))]
    pub(crate) async fn query_all_device_functions<C: ConnectionTrait>(
        device_id: Vec<Id>,
        conn: &C,
    ) -> ApiResult<Vec<DeviceFunctionModel>> {
        if device_id.is_empty() {
            return Ok(Vec::new());
        }
        DeviceFunctionEntity::find()
            .filter(DeviceFunctionColumn::Device.is_in(device_id))
            .all(conn)
            .await
            .map_err(Into::into)
    }

    pub(crate) async fn query_one_with_auth<C: ConnectionTrait>(
        user_id: Id,
        device_id: Id,
        conn: &C,
    ) -> ApiResult<DeviceWithAuth> {
        let (auth, device) = DeviceAuthorityEntity::find()
            .filter(
                DeviceAuthorityColumn::ShareId
                    .eq(user_id)
                    .and(DeviceAuthorityColumn::ShareType.eq(ShareType::User.as_ref()))
                    .and(DevicesColumn::Id.eq(device_id)),
            )
            .find_also_related(DevicesEntity)
            .one(conn)
            .await?
            .ok_or_else(|| ApiError::Device {
                device_id,
                msg: tt!("messages.device.common.device_missing", device_id = device_id),
            })?;
        match device {
            Some(device) => Ok(DeviceWithAuth { device, auth }),
            None => Err(ApiError::Device {
                device_id,
                msg: tt!("messages.device.common.device_missing", device_id = device_id),
            }),
        }
    }
    pub(crate) async fn query_one<C: ConnectionTrait>(
        user_id: Id,
        device_id: Id,
        conn: &C,
    ) -> ApiResult<DevicesModel> {
        Self::query_one_with_auth(user_id, device_id, conn).await.map(|e| e.device)
    }

    pub(crate) async fn query_io_all<C: ConnectionTrait>(device: Id, conn: &C) -> ApiResult {
        Ok(())
    }

    pub(crate) async fn query_timer_all<C: ConnectionTrait>(device: Id, conn: &C) -> ApiResult {
        Ok(())
    }

    pub(crate) async fn update_info<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        device_with_auth: DeviceWithAuth,
        info: DeviceModify,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult {
        let mut device_active = device_with_auth.device.clone().into_active_model();
        if let Some(product_id) = info.product_id {
            if Some(product_id) != device_with_auth.device.product_id {
                device_active.product_id = ActiveValue::Set(Some(product_id));
            }
        }
        if let Some(period) = info.period {
            if period != device_with_auth.device.period {
                device_active.period = ActiveValue::Set(period);
            }
        }
        if let Some(script_id) = info.script {
            if device_with_auth.device.device_type == DeviceType::LoRaNode
                && device_with_auth.device.script != info.script
            {
                let script = DecodeScriptEntity::find_by_id(script_id).one(conn).await?;
                if script.is_none() {
                    return Err(ApiError::User("invalid script".into()));
                }
                device_active.script = ActiveValue::Set(script_id.into());
                NodeInfo::update_by_eui(
                    device_with_auth.device.eui,
                    NodeInfo::script(),
                    script_id,
                    redis,
                )
                .await?;
            }
        }
        if let Some(_script) = info.reset_script {
            device_active.script = ActiveValue::Set(None);
            NodeInfo::reset_by_eui(device_with_auth.device.eui, NodeInfo::script(), redis).await?;
        }
        if device_with_auth.device.device_type == DeviceType::LoRaGate {
            let gate = device_with_auth
                .device
                .find_related(DeviceLoraGateEntity)
                .one(conn)
                .await?
                .ok_or_else(|| ApiError::User("invalid device".into()))?;
            let mut gate = gate.into_active_model();
            if let Some(region) = info.region {
                gate.region = ActiveValue::Set(region);
            }
            gate.update(conn).await?;
        }
        if device_with_auth.device.device_type == DeviceType::LoRaNode {
            let eui = device_with_auth.device.eui;
            let node = device_with_auth
                .device
                .find_related(DeviceLoraNodeEntity)
                .one(conn)
                .await?
                .ok_or_else(|| ApiError::User("invalid device".into()))?;

            let mut node = node.into_active_model();

            if let Some(name) = info.name {
                device_active.name = ActiveValue::Set(name);
            }
            if let Some(description) = info.description {
                device_active.description = ActiveValue::Set(description);
            }
            if let Some(region) = info.region {
                node.region = ActiveValue::Set(region);
                NodeInfo::update_by_eui(eui, NodeInfo::region(), region, redis).await?;
            }
            if let Some(class_c) = info.class_c {
                node.class_c = ActiveValue::Set(class_c);
                NodeInfo::update_by_eui(eui, NodeInfo::class_c(), class_c, redis).await?;
            }
            if let Some(join_type) = info.join_type {
                NodeInfo::update_by_eui(eui, NodeInfo::join_type(), join_type, redis).await?;
                node.join_type = ActiveValue::Set(join_type);
            }
            if let Some(app_eui) = info.app_eui {
                NodeInfo::update_by_eui(eui, NodeInfo::app_eui(), &app_eui, redis).await?;
                node.app_eui = ActiveValue::Set(app_eui);
            }
            if let Some(app_key) = info.app_key {
                NodeInfo::update_by_eui(eui, NodeInfo::app_key(), &app_key, redis).await?;
                node.app_key = ActiveValue::Set(app_key);
            }

            if let Some(nwk_skey) = info.nwk_skey {
                NodeInfo::update_by_eui(eui, NodeInfo::nwk_skey(), &nwk_skey, redis).await?;
                node.nwk_skey = ActiveValue::Set(nwk_skey);
            }
            if let Some(app_skey) = info.app_skey {
                NodeInfo::update_by_eui(eui, NodeInfo::app_key(), &app_skey, redis).await?;
                node.app_skey = ActiveValue::Set(app_skey);
            }
            if node.is_changed() {
                node.update(conn).await?;
            }
        }
        if device_active.is_changed() {
            device_active.update(conn).await?;
        }
        Ok(())
    }
}
