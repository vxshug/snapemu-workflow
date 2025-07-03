use crate::api::SnPath;
use crate::error::ApiResponseResult;
use crate::service::device::order::DeviceOrderService;
use crate::{get_current_user, AppState};
use axum::extract::State;
use common_define::Id;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(put_top)).routes(routes!(put_group_top))
}

/// Device top
#[utoipa::path(
    method(put),
    path = "/top/{id}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn put_top(
    State(state): State<AppState>,
    SnPath(device): SnPath<Id>,
) -> ApiResponseResult<String> {
    let user = get_current_user();
    let redis = &mut state.redis.get().await?;
    DeviceOrderService::device_top(&user, device, None, redis, &state.db).await?;
    Ok(String::from("OK").into())
}

/// The device in the device group is placed at the top
#[utoipa::path(
    method(put),
    path = "/top/{id}/{group}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn put_group_top(
    State(state): State<AppState>,
    SnPath((device, group)): SnPath<(Id, Id)>,
) -> ApiResponseResult<String> {
    let user = get_current_user();
    let redis = &mut state.redis.get().await?;
    DeviceOrderService::device_top(&user, device, Some(group), redis, &state.db).await?;
    Ok(String::from("OK").into())
}
