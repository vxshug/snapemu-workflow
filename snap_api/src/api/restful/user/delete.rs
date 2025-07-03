use crate::api::SnJson;
use crate::error::{ApiError, ApiResponseResult};
use crate::man::user_status::UserStatus;
use crate::service::user::{UserDelete, UserLang, UserReset, UserService};
use crate::utils::Checker;
use crate::{tt, AppState, AppString};
use axum::extract::State;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(request_delete)).routes(routes!(delete_user))
}

/// Request to delete a user
///
#[utoipa::path(
    method(post),
    path = "/request",
    request_body = UserReset,
    responses(
        (status = 0, description = "Success", body = str)
    ),
    tag = crate::USER_TAG
)]
pub(crate) async fn request_delete(
    lang: UserLang,
    State(state): State<AppState>,
    status: UserStatus,
    SnJson(info): SnJson<UserReset>,
) -> ApiResponseResult<AppString> {
    if !Checker::email(info.email.as_str()) {
        return Err(ApiError::User(tt!("messages.user.signup.email_format", email = info.email)));
    }

    let s = UserService::delete(status, &info, &state).await?;
    Ok(s.into())
}

/// Verify and delete user
///
#[utoipa::path(
    method(post),
    path = "",
    request_body = UserDelete,
    responses(
        (status = 0, description = "Success", body = str)
    ),
    tag = crate::USER_TAG
)]
pub(crate) async fn delete_user(
    lang: UserLang,
    State(state): State<AppState>,
    status: UserStatus,
    SnJson(info): SnJson<UserDelete>,
) -> ApiResponseResult<String> {
    if !Checker::email(info.email.as_str()) {
        return Err(ApiError::User(tt!("messages.user.signup.email_format", email = info.email)));
    }
    if info.password.len() < 8 {
        return Err(ApiError::User(tt!("messages.user.signup.password_format")));
    }

    let s = UserService::delete_user(status, &info, &state).await?;
    Ok(s.into())
}
