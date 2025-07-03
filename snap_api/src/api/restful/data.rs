use crate::api::SnPath;
use crate::error::ApiResponseResult;
use crate::service::data::query::{DataDeviceOneResponseWrap, DataDuration, DataResponseWrap};
use crate::service::data::DataService;
use crate::service::device::DeviceService;
use crate::{get_current_user, AppState};
use axum::extract::State;
use common_define::Id;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_hour_data))
        .routes(routes!(get_day_data))
        .routes(routes!(get_week_data))
        .routes(routes!(get_last_data))
    // .route("/:device/range", get(get_range_data))
}

/// Get 1 hour of data
#[utoipa::path(
    method(get),
    path = "/{id}/hour",
    params(
            ("id" = i32, Path, description = "Todo database id")
    ),
    security(
        (), // <-- make optional authentication
    ),
    responses(
        (status = OK, description = "Success", body = str, content_type = "text/plain")
    ),
    tag = crate::DATA_TAG
)]
async fn get_hour_data(
    State(state): State<AppState>,
    SnPath(device): SnPath<Id>,
) -> ApiResponseResult<DataResponseWrap> {
    let user = get_current_user();
    let device_db = DeviceService::query_one(user.id, device, &state.db).await?;
    let data =
        DataService::query_duration_data(device, DataDuration::Hour, &state)
            .await?;
    Ok(data.into())
}

/// Get 1 day of data
#[utoipa::path(
    method(get),
    path = "/{id}/day",
    params(
            ("id" = i32, Path, description = "Todo database id")
    ),
    security(
        (), // <-- make optional authentication
    ),
    responses(
        (status = OK, description = "Success", body = str, content_type = "text/plain")
    ),
    tag = crate::DATA_TAG
)]
async fn get_day_data(
    State(state): State<AppState>,
    SnPath(device): SnPath<Id>,
) -> ApiResponseResult<DataResponseWrap> {
    let user = get_current_user();
    let device_db = DeviceService::query_one(user.id, device, &state.db).await?;
    let data =
        DataService::query_duration_data(device, DataDuration::Day, &state)
            .await?;
    Ok(data.into())
}

/// Get 1 week of data
#[utoipa::path(
    method(get),
    path = "/{id}/week",
    params(
            ("id" = i32, Path, description = "Todo database id")
    ),
    security(
        (), // <-- make optional authentication
    ),
    responses(
        (status = OK, description = "Success", body = str, content_type = "text/plain")
    ),
    tag = crate::DATA_TAG
)]
async fn get_week_data(
    State(state): State<AppState>,
    SnPath(device): SnPath<Id>,
) -> ApiResponseResult<DataResponseWrap> {
    let user = get_current_user();
    let device_db = DeviceService::query_one(user.id, device, &state.db).await?;
    let data =
        DataService::query_duration_data(device, DataDuration::Week, &state)
            .await?;
    Ok(data.into())
}

/// Get the last data
#[utoipa::path(
    method(get),
    path = "/{id}/last",
    params(
            ("id" = i32, Path, description = "Todo database id")
    ),
    security(
        (), // <-- make optional authentication
    ),
    responses(
        (status = OK, description = "Success", body = str, content_type = "text/plain")
    ),
    tag = crate::DATA_TAG
)]
async fn get_last_data(
    State(state): State<AppState>,
    SnPath(device): SnPath<Id>,
) -> ApiResponseResult<DataDeviceOneResponseWrap> {
    let user = get_current_user();

    let device_db = DeviceService::query_one(user.id, device, &state.db).await?;
    let data = DataService::query_last(&device_db, &state).await?;
    Ok(data.into())
}

#[derive(Debug, serde::Deserialize)]
struct QueryRangeParams {
    s: u64,
    e: u64,
    id: Option<u64>,
    c: Option<u64>,
}
//
// async fn get_range_data(
//     mut conn: DatabaseConnection,
//     Query(params): Query<QueryRangeParams>,
//     Path(device): Path<Id>,
// ) -> ApiResponseResult<DataResponseWrap> {
//     let user = get_current_user();
//
//     snap_log::log::warn!("{:?}", params);
//     if params.s > params.e {
//         return Err(ApiError::User(
//             t!("messages.user.data.time_start_end", locale = user.lang.as_ref())
//         ))
//     }
//     if (params.e - params.s) > (60 * 60 * 24 * 8) {
//         return Err(ApiError::User(
//             t!("messages.user.data.time_range", locale = user.lang.as_ref())
//         ))
//     }
//     let start = Time::form_sec(params.s);
//     let end = Time::form_sec(params.e);
//     let count = params.c.unwrap_or(24);
//     let data_id = params.id.map(|id| id as i32);
//     let device_db = DeviceService::query_one(user.id, device, conn.as_mut()).await?;
//     let data = DataService::query_range_data(&user, device, device_db.script, data_id, start,  end, count, conn.as_mut()).await?;
//     Ok(data.into())
// }
