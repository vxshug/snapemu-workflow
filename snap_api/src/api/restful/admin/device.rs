use crate::api::{SnJson, SnPath};
use crate::error::{ApiError, ApiResponseResult};
use crate::AppState;
use axum::extract::{Query, State};
use axum::routing::get;
use common_define::db::{
    DecodeMap, DecodeScriptEntity, DeviceLoraNodeEntity, DevicesColumn, DevicesEntity, Eui, Key,
    SnapDeviceEntity, UsersEntity,
};
use common_define::lora::{LoRaJoinType, LoRaRegion};
use common_define::product::{DeviceType, ProductType};
use common_define::time::Timestamp;
use common_define::Id;
use device_info::lorawan::{GatewayInfo, NodeInfo};
use device_info::snap::SnapDeviceInfo;
use sea_orm::{
    ActiveModelTrait, ActiveValue, EntityTrait, IntoActiveModel, ModelTrait, PaginatorTrait,
    QueryOrder,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::warn;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .route("/", get(get_all_devices))
        .route("/count", get(get_devices_count))
        .route("/info/:id", get(get_device_info).put(put_device_info))
}

#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
struct DeviceQuery {
    #[param(value_type = Option<u64>, example = 0, minimum = 0, default = 0)]
    page: Option<u64>,
}

#[derive(Serialize)]
struct DevicePages {
    page: u64,
    count: u64,
    devices: Vec<DevicePageItem>,
}

#[derive(Serialize)]
struct DevicePageItem {
    pub id: Id,
    pub eui: Eui,
    pub name: String,
    pub description: String,
    pub creator: Id,
    pub enable: bool,
    pub period: i32,
    pub script: Option<Id>,
    pub device_type: DeviceType,
    pub active_time: Option<Timestamp>,
    pub create_time: Timestamp,
}

#[derive(Serialize)]
struct DeviceCountBody {
    count: u64,
}

#[derive(Serialize)]
struct DeviceScriptBody {
    pub id: Id,
    pub script: String,
    pub lang: String,
    pub owner: Id,
    pub name: String,
    pub map: DecodeMap,
    pub create_time: Timestamp,
    pub modify_time: Timestamp,
}

#[derive(Serialize)]
struct DeviceCreatorBody {
    username: String,
}

#[derive(Serialize)]

struct DeviceInfoBody {
    pub id: Id,
    pub eui: Eui,
    pub name: String,
    pub description: String,
    pub creator: Option<DeviceCreatorBody>,
    pub creator_id: Id,
    pub enable: bool,
    pub period: i32,
    pub script: Option<DeviceScriptBody>,
    pub script_id: Option<Id>,
    pub device_type: DeviceType,
    pub active_time: Option<Timestamp>,
    pub create_time: Timestamp,
    pub info: Option<DeviceTypeInfoBody>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum DeviceTypeInfoBody {
    LoRaNode(NodeInfo),
    LoRaGateway(GatewayInfo),
    Snap(SnapDeviceInfo),
}

/// Device information that can be modified
#[derive(Deserialize, Serialize, Default)]
struct DeviceChangeBody {
    /// device name.
    pub name: Option<String>,
    /// Device description information.
    pub description: Option<String>,
    /// false disables the device, and true indicates that the device is available
    pub enable: Option<bool>,
    /// LoRaNode. id of the decoding script. If null is set, the decoding script is not used
    pub script_id: Option<u64>,
    /// LoRaNode. Regional parameter, Enumerated value: EU868, US915, CN779, EU433, AU915, CN470, AS923_1, AS923_2, AS923_3, KR920, IN865, RU864
    // #[schema(pattern="^EU868|US915|CN779|EU433|AU915|CN470|AS923_1|AS923_2|AS923_3|KR920|IN865|RU864$")]
    pub region: Option<String>,
    /// LoRaNode. Enumerated value: OTAA, ABP
    // #[schema(pattern="^OTAA|ABP$")]
    pub join_type: Option<String>,
    /// LoRaNode. app_eui
    // #[schema(max_length = 16, min_length=16, pattern="^[0-9A-F]{16}$")]
    pub app_eui: Option<String>,
    /// LoRaNode. otaa access key
    // #[schema(max_length = 32, min_length=32, pattern="^[0-9A-F]{32}$")]
    pub app_key: Option<String>,
    /// LoRaNode. nwk_skey
    // #[schema(max_length = 32, min_length=32, pattern="^[0-9A-F]{32}$")]
    pub nwk_skey: Option<String>,
    /// LoRaNode. app_skey
    // #[schema(max_length = 32, min_length=32, pattern="^[0-9A-F]{32}$")]
    pub app_skey: Option<String>,
    /// LoRaNode.
    pub class_b: Option<bool>,
    /// LoRaNode.
    pub class_c: Option<bool>,
    /// LoRaNode.
    pub adr: Option<bool>,
    /// LoRaNode.
    pub rx1_delay: Option<i16>,
    /// LoRaNode.
    pub des_rx1_delay: Option<i16>,
    /// LoRaNode.
    pub rx1_dro: Option<i16>,
    /// LoRaNode.
    pub des_rx1_dro: Option<i16>,
    /// LoRaNode.
    pub rx2_dr: Option<i16>,
    /// LoRaNode.
    pub des_rx2_dr: Option<i16>,
    /// LoRaNode.
    pub rx2_freq: Option<i32>,
    /// LoRaNode.
    pub des_rx2_freq: Option<i32>,
    /// LoRaNode.
    pub d_retry: Option<i16>,
    /// LoRaNode.
    pub c_retry: Option<i16>,
    /// LoRaNode. Custom, Monitor, Controller, Gate
    // #[schema(pattern="^Custom|Monitor|Controller|Gate$")]
    pub product_type: Option<ProductType>,
    /// LoRaNode.
    pub dutycyle: Option<i32>,
    /// LoRaNode.
    pub up_confirm: Option<bool>,
    /// LoRaNode.
    pub up_dr: Option<i16>,
    /// LoRaNode.
    pub time_zone: Option<i32>,
    /// Snap.
    // #[schema(max_length = 32, min_length=32, pattern="^[0-9A-F]{32}$")]
    pub key: Option<String>,
}

#[derive(OpenApi)]
#[openapi(paths(get_all_devices, get_devices_count, get_device_info, put_device_info))]
pub struct DeviceApi;

///
/// Number of all devices
#[utoipa::path(
    get,
    path = "/device/count",
    params(
        DeviceQuery,
    ),
    responses(
            (status = 0, description = "device count"),
    )
)]
async fn get_devices_count(State(state): State<AppState>) -> ApiResponseResult<DeviceCountBody> {
    let count = DevicesEntity::find().count(&state.db).await?;
    Ok(DeviceCountBody { count }.into())
}

///
/// Obtain device information by paging
#[utoipa::path(
    get,
    path = "/device",
    params(
        DeviceQuery,
    ),
    responses(
            (status = 0, description = "device page"),
    )
)]
async fn get_all_devices(
    State(state): State<AppState>,
    Query(page): Query<DeviceQuery>,
) -> ApiResponseResult<DevicePages> {
    let page = page.page.unwrap_or(0);
    let device_pages =
        DevicesEntity::find().order_by_asc(DevicesColumn::Id).paginate(&state.db, 50);
    let page_count = device_pages.num_pages().await?;
    if page_count < page {
        return Err(ApiError::User(
            format!("page {} is more than max {}", page, page_count).into(),
        ));
    }
    let devices = device_pages.fetch_page(page).await?;

    let v = devices
        .into_iter()
        .map(|it| DevicePageItem {
            id: it.id,
            eui: it.eui,
            name: it.name,
            description: it.description,
            creator: it.creator,
            enable: it.enable,
            period: it.period,
            script: it.script,
            device_type: it.device_type,
            active_time: it.active_time,
            create_time: it.create_time,
        })
        .collect();
    Ok(DevicePages { page, count: page_count, devices: v }.into())
}

///
/// Get all information about the device
#[utoipa::path(
    get,
    path = "/device/info/{id}",
    params(
        ("id", description = "Device id"),
    ),
    responses(
            (status = 0, description = "information"),
    )
)]
async fn get_device_info(
    State(state): State<AppState>,
    SnPath(id): SnPath<Id>,
) -> ApiResponseResult<DeviceInfoBody> {
    let device = DevicesEntity::find_by_id(id).one(&state.db).await?.ok_or_else(|| {
        warn!("Failed to find device with id {}", id);
        ApiError::User("Device not found".into())
    })?;

    let script = match device.script {
        Some(script) => DecodeScriptEntity::find_by_id(script).one(&state.db).await?.map(|it| {
            DeviceScriptBody {
                id: it.id,
                script: it.script,
                lang: it.lang,
                owner: it.owner,
                name: it.name,
                map: it.map,
                create_time: it.create_time,
                modify_time: it.modify_time,
            }
        }),
        None => None,
    };

    let creator = UsersEntity::find_by_id(device.creator)
        .one(&state.db)
        .await?
        .map(|user| DeviceCreatorBody { username: user.user_login });

    let conn = &mut state.redis.get().await?;
    let info = match device.device_type {
        DeviceType::LoRaNode => match NodeInfo::load_by_eui(device.eui, conn).await? {
            None => {
                let node =
                    device.find_related(DeviceLoraNodeEntity).one(&state.db).await?.ok_or_else(
                        || {
                            warn!("Failed to find device with id {}", id);
                            ApiError::User("Device not found".into())
                        },
                    )?;
                DeviceTypeInfoBody::LoRaNode(
                    NodeInfo::register_to_redis(node, device.clone(), conn).await?,
                )
            }
            Some(info) => DeviceTypeInfoBody::LoRaNode(info),
        },
        DeviceType::LoRaGate => match GatewayInfo::load(device.eui, conn).await? {
            None => {
                let info = GatewayInfo::new(device.id, 0, 0, Timestamp::now(), None, None);
                info.register(device.eui, conn).await?;
                DeviceTypeInfoBody::LoRaGateway(info)
            }
            Some(info) => DeviceTypeInfoBody::LoRaGateway(info),
        },
        DeviceType::MQTT => return Err(ApiError::User("The device is not supported".into())),
        DeviceType::Snap => match SnapDeviceInfo::load(device.eui, conn).await? {
            None => {
                let snap = device.find_related(SnapDeviceEntity).one(&state.db).await?.ok_or_else(
                    || {
                        warn!("Failed to find device with id {}", id);
                        ApiError::User("Device not found".into())
                    },
                )?;
                let snap = SnapDeviceInfo::new(
                    device.id,
                    snap.key,
                    Some(Timestamp::now()),
                    0,
                    None,
                    device.script,
                    None,
                );
                snap.register(device.eui, conn).await?;
                DeviceTypeInfoBody::Snap(snap)
            }
            Some(info) => DeviceTypeInfoBody::Snap(info),
        },
    };

    Ok(DeviceInfoBody {
        id,
        eui: device.eui,
        name: device.name,
        description: device.description,
        creator,
        creator_id: device.creator,
        enable: device.enable,
        period: device.period,
        script,
        script_id: device.script,
        device_type: device.device_type,
        active_time: device.active_time,
        create_time: device.create_time,
        info: Some(info),
    }
    .into())
}

///
/// Modifying Device Information
#[utoipa::path(
    put,
    path = "/device/info/{id}",
    params(
        ("id", Path, description = "Device id"),
    ),
    responses(
            (status = 0, description = "information"),
    )
)]
async fn put_device_info(
    State(state): State<AppState>,
    SnPath(id): SnPath<Id>,
    SnJson(info): SnJson<DeviceChangeBody>,
) -> ApiResponseResult<DeviceChangeBody> {
    let mut body = DeviceChangeBody::default();
    let device = DevicesEntity::find_by_id(id).one(&state.db).await?.ok_or_else(|| {
        warn!("Failed to find device with id {}", id);
        ApiError::User("Device not found".into())
    })?;

    match device.device_type {
        DeviceType::LoRaNode => {
            let node = device
                .find_related(DeviceLoraNodeEntity)
                .one(&state.db)
                .await?
                .ok_or_else(|| ApiError::User("invalid device".into()))?;
            let eui = node.dev_eui;
            let mut node = node.into_active_model();
            let redis = &mut state.redis.get().await?;
            macro_rules! parse_from_info {
                ($(($k:ident, $t:ty)),*) => {
                    {
                        $(let $k = match info.$k {
                            Some($k) => {
                                let r = Some(<$t>::from_str(&$k).map_err(|_| ApiError::User(concat!("invalid ", stringify!($k)).into()))?);
                                body.$k = Some($k);
                                r
                            },
                            None => None,
                        };)*
                        $(if let Some($k) = $k {
                            node.$k = ActiveValue::Set($k);
                            NodeInfo::update_by_eui(eui, NodeInfo::$k(), $k, redis).await?;
                        })*
                    }
                };
            }
            parse_from_info!(
                (region, LoRaRegion),
                (join_type, LoRaJoinType),
                (app_eui, Eui),
                (app_key, Key),
                (nwk_skey, Key),
                (app_skey, Key)
            );

            macro_rules! update_node {
                ($i:ident) => {
                    if let Some($i) = info.$i {
                        NodeInfo::update_by_eui(eui, NodeInfo::$i(), &$i, redis).await?;
                        node.$i = ActiveValue::Set($i);
                        body.$i = Some($i);
                    }
                };
            }
            update_node!(class_b);
            update_node!(adr);
            update_node!(rx1_delay);
            update_node!(des_rx1_delay);
            update_node!(rx1_dro);
            update_node!(des_rx1_dro);
            update_node!(rx2_dr);
            update_node!(des_rx2_dr);
            update_node!(rx2_freq);
            update_node!(des_rx2_freq);
            update_node!(d_retry);
            update_node!(c_retry);
            update_node!(product_type);
            update_node!(dutycyle);
            update_node!(up_confirm);
            update_node!(up_dr);
            update_node!(time_zone);
            if node.is_changed() {
                node.update(&state.db).await?;
            }
        }
        DeviceType::LoRaGate => {}
        DeviceType::MQTT => {}
        DeviceType::Snap => {
            let snap = device
                .find_related(SnapDeviceEntity)
                .one(&state.db)
                .await?
                .ok_or_else(|| ApiError::User("invalid device".into()))?;
            let mut snap = snap.into_active_model();
            let key = match info.key {
                Some(key) => {
                    let r = Some(
                        Key::from_str(&key).map_err(|_| ApiError::User("invalid key".into()))?,
                    );
                    body.key = Some(key);
                    r
                }
                None => None,
            };
            if let Some(key) = key {
                let redis = &mut state.redis.get().await?;
                SnapDeviceInfo::update_by_eui(device.eui, SnapDeviceInfo::key(), &key, redis)
                    .await?;
                snap.key = ActiveValue::Set(key);
            }
        }
    }

    Ok(body.into())
}
