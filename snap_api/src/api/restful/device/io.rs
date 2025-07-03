use axum::extract::State;
use axum::{extract::Path, Router};
use common_define::Id;

use crate::{error::ApiResponseResult, get_current_user, AppState};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub(crate) struct GPIOTimer {
    pub(crate) num: i32,
    pub(crate) pin: i32,
    pub(crate) enable: bool,
    pub(crate) action: bool,
    pub(crate) hour: i32,
    pub(crate) minute: i32,
    pub(crate) repeat: i32,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub(crate) struct GPIOItem {
    pub(crate) pin: i32,
    pub(crate) value: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
struct IoRequest {
    io: Option<Vec<GPIOItem>>,
    timer: Option<GPIOTimer>,
}

async fn get_io(State(state): State<AppState>, Path(device): Path<Id>) -> ApiResponseResult {
    // let user = get_current_user();
    // DeviceService::query_one(user.id, device, &state.db).await?;
    // let ios = DeviceService::query_io_all(device, &state.db).await?;
    // let io: Vec<_> = ios.into_iter().map(|item| DeviceIoResp {
    //     pin: item.pin,
    //     modify: item.modify,
    //     output: item.output,
    //     value: item.value,
    //     update_time: item.update_time,
    // }).collect();
    Ok(().into())
}

async fn get_timer(State(state): State<AppState>, Path(device): Path<Id>) -> ApiResponseResult {
    let user = get_current_user();

    // DeviceService::query_one(user.id, device, &state.db).await?;
    // let timers = DeviceService::query_timer_all(device, &state.db).await?;
    // let timer: Vec<_> = timers.into_iter().map(|item| DeviceTimerResp {
    //     num: item.num,
    //     enable: item.enable,
    //     pin: item.pin,
    //     action: item.action,
    //     hour: item.hour,
    //     minute: item.minute,
    //     repeat: item.repeat,
    // }).collect();
    Ok(().into())
}
