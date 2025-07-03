use crate::api::SnJson;
use crate::error::ApiResponseResult;
use crate::service::user::{UserPutInfo, UserRespInfo, UserService};
use crate::{get_current_user, AppState};
use axum::extract::State;

/// Modify user information
///
#[utoipa::path(
    method(put),
    path = "/info",
    request_body = UserPutInfo,
    responses(
        (status = 0, description = "Success", body = str)
    ),
    tag = crate::USER_TAG
)]
pub(crate) async fn info(
    State(state): State<AppState>,
    SnJson(info): SnJson<UserPutInfo>,
) -> ApiResponseResult<String> {
    let user = get_current_user();
    UserService::info(&user, info, &state).await?;
    Ok(String::new().into())
}

/// Get user information
///
#[utoipa::path(
    method(get),
    path = "/info",
    responses(
        (status = 0, description = "Success", body = UserRespInfo)
    ),
    tag = crate::USER_TAG
)]
pub(crate) async fn get_info(State(state): State<AppState>) -> ApiResponseResult<UserRespInfo> {
    let user = get_current_user();

    Ok(UserService::get_info(&user, &state).await?.into())
}
