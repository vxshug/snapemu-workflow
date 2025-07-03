use crate::error::ApiResponseResult;
use crate::service::device::device::DeviceResp;
use crate::service::device::group::DeviceGroupResp;
use crate::service::device::DeviceService;
use crate::{get_current_user, AppState};
use axum::extract::State;
use axum::routing::get;
use axum::Router;

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/", get(get_all_device))
}

async fn get_all_device(State(state): State<AppState>) -> ApiResponseResult<DeviceGroupResp> {
    let user = get_current_user();
    let redis = &mut state.redis.get().await?;
    let all = DeviceService::query_all(user.id, redis, &state.db).await?;
    let devices: Vec<DeviceResp> = all
        .into_iter()
        .map(|item| DeviceResp {
            id: item.id.into(),
            eui: Some(item.eui),
            name: item.name.into(),
            blue_name: None,
            online: None,
            period: None,
            battery: None,
            data: None,
            charge: None,
            description: None,
            info: None,
            source: None,
            device_type: None,
            product_type: None,
            create_time: item.create_time.into(),
            active_time: item.active_time,
            script: None,
            product_id: None,
            product_url: None,
            group: None,
        })
        .collect();
    let res = DeviceGroupResp {
        id: None,
        name: None,
        device_count: Some(devices.len() as i64),
        default_group: None,
        offset: Some(0),
        description: None,
        create_time: None,
        devices: Some(devices),
        ..Default::default()
    };
    Ok(res.into())
}
