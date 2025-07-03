use crate::api::{SnJson, SnPath};
use crate::error::{ApiError, ApiResponseResult};
use crate::service::device::group::{
    DeviceGroupResp, DeviceGroupService, ReqDeviceGroup, ReqPutDeviceGroup,
};
use crate::{get_current_user, tt, AppState, AppString};
use axum::extract::State;
use common_define::Id;
use sea_orm::TransactionTrait;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(post_group, all_group))
        .routes(routes!(get_group, put_group, delete_group))
        .routes(routes!(delete_group_device))
}

/// Create a device group
#[utoipa::path(
    method(post),
    path = "",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn post_group(
    State(app_state): State<AppState>,
    group: SnJson<ReqDeviceGroup>,
) -> ApiResponseResult<AppString> {
    let user = get_current_user();
    let redis = &mut app_state.redis.get().await?;
    DeviceGroupService::create_group(group.0, &user, redis, &app_state.db).await?;
    Ok(tt!("messages.device.group.create_success").into())
}

/// Get a device group
#[utoipa::path(
    method(get),
    path = "/{group}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn get_group(
    State(state): State<AppState>,
    SnPath(id): SnPath<Id>,
) -> ApiResponseResult<DeviceGroupResp> {
    let user = get_current_user();
    let mut redis = state.redis.get().await?;
    let group = DeviceGroupService::select_by_group_id(&user, id, &mut redis, &state.db).await?;
    let groups = DeviceGroupService::select_one(&user, &state, group).await?;
    Ok(groups.into())
}

/// Get all device groups
#[utoipa::path(
    method(get),
    path = "",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn all_group(State(state): State<AppState>) -> ApiResponseResult<Vec<DeviceGroupResp>> {
    let user = get_current_user();

    let groups = DeviceGroupService::select_all(&user, &state.db).await?;
    Ok(groups.into())
}

/// Deleting a device group
#[utoipa::path(
    method(delete),
    path = "/{group}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn delete_group(
    State(state): State<AppState>,
    SnPath(id): SnPath<Id>,
) -> ApiResponseResult<AppString> {
    let mut redis = state.redis.get().await?;
    state
        .db
        .transaction::<_, _, ApiError>(|ctx| {
            Box::pin(async move {
                let user = get_current_user();
                DeviceGroupService::delete_one(&user, id, &mut redis, ctx).await?;
                Ok(())
            })
        })
        .await?;
    Ok(tt!("messages.device.group.delete_success").into())
}

/// Device removal group
#[utoipa::path(
    method(delete),
    path = "/{group}/{id}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn delete_group_device(
    State(state): State<AppState>,
    SnPath((group, device)): SnPath<(Id, Id)>,
) -> ApiResponseResult<AppString> {
    let mut redis = state.redis.get().await?;
    let user = get_current_user();
    DeviceGroupService::unlink(device, user.id, group, &mut redis, &state.db).await?;
    Ok(tt!("messages.device.group.delete_device").into())
}

/// Modify a device group
#[utoipa::path(
    method(put),
    path = "/{group}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn put_group(
    State(state): State<AppState>,
    SnPath(group): SnPath<Id>,
    SnJson(req): SnJson<ReqPutDeviceGroup>,
) -> ApiResponseResult<AppString> {
    let mut redis = state.redis.get().await?;
    state
        .db
        .transaction::<_, _, ApiError>(|ctx| {
            Box::pin(async move {
                let user = get_current_user();
                DeviceGroupService::update_group(group, &user, req, &mut redis, ctx).await?;
                Ok(())
            })
        })
        .await?;
    Ok(tt!("messages.device.group.link_device").into())
}
