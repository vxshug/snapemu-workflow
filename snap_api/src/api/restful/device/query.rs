use crate::api::SnJson;
use crate::error::{ApiError, ApiResponseResult};
use crate::man::DeviceQueryClient;
use crate::service::device::define::DeviceParameter;
use crate::service::lorawan::JoinParam;
use crate::{tt, AppState};
use axum::extract::State;
use common_define::db::{
    DeviceLoraNodeColumn, DeviceLoraNodeEntity, DevicesColumn, DevicesEntity, Eui, LoRaAddr,
};
use common_define::lora::{LoRaJoinType, LoRaRegion};
use common_define::product::DeviceType;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(post_query))
}

#[derive(Serialize, Deserialize)]
pub(crate) struct QueryDeviceReq {
    eui: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct QueryRegister {
    dev_eui: Option<String>,
    gateway: Option<String>,
    dev_addr: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct DeviceRsp {
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sensor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) eui: Option<String>,
    pub(crate) device_type: DeviceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) region: Option<LoRaRegion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) join_type: Option<LoRaJoinType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) join_parameter: Option<JoinParam>,
}

#[derive(Serialize, Deserialize)]
struct LoRaNode {
    region: LoRaRegion,
    sensor: String,
    join_type: LoRaJoinType,
    join_parameter: Value,
}

#[derive(Serialize, Deserialize)]
struct LoRaGate {
    region: LoRaRegion,
    eui: String,
}

#[utoipa::path(
    method(post),
    path = "",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
pub(crate) async fn post_query(
    SnJson(req): SnJson<QueryDeviceReq>,
) -> ApiResponseResult<DeviceRsp> {
    let req_eui = match req.eui.strip_prefix("https://") {
        Some(v) => v,
        None => req.eui.as_str(),
    };
    if req_eui.len() != 32 || !req_eui.is_ascii() {
        return Err(ApiError::User(tt!("messages.device.common.eui_format")));
    }
    let eui = &req_eui[16..];
    let device = DeviceQueryClient::query_eui(eui)
        .await?
        .ok_or(ApiError::User(tt!("messages.device.common.eui_not_found", eui = eui)))?;

    match device.device_type {
        DeviceType::Snap | DeviceType::MQTT => Ok(DeviceRsp {
            name: device.name,
            sensor: None,
            device_type: device.device_type,
            eui: Some(req.eui),
            region: Some(LoRaRegion::CN470),
            join_type: None,
            join_parameter: None,
        }
        .into()),
        DeviceType::LoRaGate => {
            if let DeviceParameter::Gate(gate) = device.parameter {
                Ok(DeviceRsp {
                    name: device.name,
                    sensor: None,
                    device_type: device.device_type,
                    eui: Some(gate.eui),
                    region: Some(gate.region),
                    join_type: None,
                    join_parameter: None,
                }
                .into())
            } else {
                Err(ApiError::User(tt!("messages.device.common.eui_found_err", eui = eui)))
            }
        }
        DeviceType::LoRaNode => {
            if let DeviceParameter::Device(node) = device.parameter {
                Ok(DeviceRsp {
                    name: device.name,
                    sensor: node.sensor.into(),
                    eui: Some(node.join_parameter.dev_eui.clone()),
                    device_type: device.device_type,
                    region: node.region.into(),
                    join_type: node.join_type.into(),
                    join_parameter: Some(JoinParam {
                        app_skey: Some(node.join_parameter.app_skey),
                        nwk_skey: Some(node.join_parameter.nwk_skey),
                        dev_addr: Some(node.join_parameter.dev_addr),
                        app_key: Some(node.join_parameter.app_key),
                        app_eui: Some(node.join_parameter.app_eui),
                        dev_eui: Some(node.join_parameter.dev_eui),
                    }),
                }
                .into())
            } else {
                Err(ApiError::User(tt!("messages.device.common.eui_found_err", eui = eui)))
            }
        }
    }
}

#[utoipa::path(
    method(get),
    path = "",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
pub async fn register_query(
    State(state): State<AppState>,
    SnJson(req): SnJson<QueryRegister>,
) -> ApiResponseResult<()> {
    if let Some(eui) = req.dev_eui {
        let eui: Eui = eui.parse()?;
        let device =
            DevicesEntity::find().filter(DevicesColumn::Eui.eq(eui)).one(&state.db).await?;
        if device.is_some() {
            return Err(ApiError::User(tt!("messages.device.common.device_already", eui = eui)));
        }
    }
    if let Some(gateway) = req.gateway {
        let gateway: Eui = gateway.parse()?;
        let device =
            DevicesEntity::find().filter(DevicesColumn::Eui.eq(gateway)).one(&state.db).await?;
        if device.is_some() {
            return Err(ApiError::User(tt!(
                "messages.device.common.device_already",
                eui = gateway
            )));
        }
    }

    if let Some(addr) = req.dev_addr {
        let addr: LoRaAddr = addr.parse()?;

        let node = DeviceLoraNodeEntity::find()
            .filter(DeviceLoraNodeColumn::DevAddr.eq(addr))
            .one(&state.db)
            .await?;

        if node.is_some() {
            return Err(ApiError::User(tt!("messages.device.lora.dev_addr_already", addr = addr)));
        }
    }

    Ok(().into())
}
