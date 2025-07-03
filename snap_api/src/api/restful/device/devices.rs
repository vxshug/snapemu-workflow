use crate::api::{SnJson, SnPath};
use axum::extract::State;
use common_define::product::DeviceType;
use common_define::Id;
use sea_orm::TransactionTrait;
use tracing::instrument;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use common_define::time::Timestamp;
use crate::service::device::device::{
    DeviceCreate, DeviceInfo, DeviceModify, DeviceResp, DeviceSource, DeviceWithAuth,
    MQTTDeviceInfo,
};

use crate::cache::DeviceCache;
use crate::error::{ApiError, ApiResponseResult};
use crate::service::device::group::{cmp_period, DeviceGroupResp, DeviceGroupService};
use crate::service::device::DeviceService;
use crate::service::lorawan::{LoRaGateService, LoRaNodeService};
use crate::service::mqtt::MQTTService;
use crate::service::snap::SnapDeviceService;
use crate::{get_current_user, tt, AppState, AppString, GLOBAL_PRODUCT_NAME};

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_all_device, post_device)).routes(routes!(
        get_device,
        put_device,
        delete_device
    ))
}

/// Get all devices
///
#[utoipa::path(
    method(get),
    path = "",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn get_all_device(State(state): State<AppState>) -> ApiResponseResult<DeviceGroupResp> {
    let user = get_current_user();
    let redis = &mut state.redis.get().await?;
    let mut all = DeviceGroupService::select_default_group(&user, redis, &state).await?;
    all.id = None;
    all.name = None;
    all.default_group = None;
    all.description = None;
    Ok(all.into())
}

/// Get one device
///
#[utoipa::path(
    method(get),
    path = "/{id}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn get_device(
    State(state): State<AppState>,
    SnPath(id): SnPath<Id>,
) -> ApiResponseResult<DeviceResp> {
    let user = get_current_user();
    let conn = &state.db;
    let DeviceWithAuth { auth, device } =
        DeviceService::query_one_with_auth(user.id, id, conn).await?;
    let info = match device.device_type {
        DeviceType::MQTT => {
            let s = MQTTService::get(id, conn).await?;
            DeviceInfo::MQTT(MQTTDeviceInfo::new(id, s.eui, s.username, s.password))
        }
        DeviceType::LoRaNode => {
            let info = LoRaNodeService::get_lora_node(id, conn).await?;
            DeviceInfo::LoRaNode(info.into())
        }
        DeviceType::LoRaGate => {
            let info = LoRaGateService::get_gateway(id, conn).await?;
            DeviceInfo::LoRaGate(info.into())
        }
        DeviceType::Snap => {
            let info = SnapDeviceService::get_device(id, conn).await?;
            DeviceInfo::Snap(info.into())
        }
    };

    let group = DeviceGroupService::query_by_device(device.id, &user, conn).await?;
    let group = group
        .into_iter()
        .map(|item| DeviceGroupResp {
            id: item.id.into(),
            name: item.name.into(),
            ..Default::default()
        })
        .collect();
    let product = GLOBAL_PRODUCT_NAME.get_by_id(device.product_id);
    let (product_id, product_url) = match product {
        Some(p) => (Some(p.id), Some(p.image)),
        None => (None, None),
    };
    let resp = DeviceResp {
        id: device.id.into(),
        eui: Some(device.eui),
        name: device.name.into(),
        blue_name: None,
        online: Some(cmp_period(device.period, Timestamp::now().timestamp_millis(), &device.active_time)),
        period: device.period.into(),
        charge: None,
        battery: None,
        description: device.description.into(),
        info: info.into(),
        source: DeviceSource {
            share_type: auth.share_type,
            owner: auth.owner,
            manager: auth.manager,
            modify: auth.modify,
            delete: auth.delete,
            share: auth.share,
        }
        .into(),
        device_type: device.device_type.into(),
        product_type: None,
        create_time: device.create_time.into(),
        active_time: device.active_time,
        data: None,
        script: device.script,
        product_id,
        product_url,
        group: Some(group),
    };
    Ok(resp.into())
}

/// Create a new device
///
#[utoipa::path(
    method(post),
    path = "",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
#[instrument(skip(state))]
async fn post_device(
    State(state): State<AppState>,
    SnJson(req): SnJson<DeviceCreate>,
) -> ApiResponseResult<AppString> {
    let user = get_current_user();
    if req.eui.is_none() {
        return Err(ApiError::User(tt!("messages.device.common.eui_missing")));
    }

    let redis = state.redis.clone();

    state
        .db
        .transaction::<_, _, ApiError>(|ctx| {
            Box::pin(async move {
                let mut redis = redis.get().await?;
                DeviceService::new_device(&user, req, &mut redis, ctx).await?;
                DeviceCache::delete_by_user_id(user.id, &mut redis).await?;
                Ok(())
            })
        })
        .await?;

    Ok(tt!("messages.device.create_success").into())
}

/// Deleting a device
///
#[utoipa::path(
    method(delete),
    path = "/{id}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn delete_device(
    State(state): State<AppState>,
    SnPath(id): SnPath<Id>,
) -> ApiResponseResult<AppString> {
    let redis = state.redis.clone();
    state
        .db
        .transaction::<_, _, ApiError>(|ctx| {
            Box::pin(async move {
                let user = get_current_user();
                let mut redis = redis.get().await?;
                DeviceService::delete(id, &user, &mut redis, ctx).await?;
                DeviceCache::delete_by_user_id(user.id, &mut redis).await?;
                Ok(())
            })
        })
        .await?;
    Ok(tt!("messages.device.delete_success").into())
}

/// Modify device information
///
#[utoipa::path(
    method(put),
    path = "/{id}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn put_device(
    State(state): State<AppState>,
    SnPath(id): SnPath<Id>,
    SnJson(req): SnJson<DeviceModify>,
) -> ApiResponseResult<String> {
    let user = get_current_user();
    let device_with_auth = DeviceService::query_one_with_auth(user.id, id, &state.db).await?;
    let redis = state.redis.clone();
    state
        .db
        .transaction::<_, _, ApiError>(|ctx| {
            Box::pin(async move {
                let mut redis = redis.get().await?;
                DeviceService::update_info(device_with_auth, req, &mut redis, ctx).await?;
                DeviceCache::delete_by_user_id(user.id, &mut redis).await?;
                Ok(())
            })
        })
        .await?;

    Ok(String::new().into())
}
