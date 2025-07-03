use crate::api::SnJson;
use crate::error::{ApiError, ApiResponseResult};
use crate::man::user_status::UserStatus;
use crate::service::user::{UserInfo, UserLang, UserReset, UserResetPassword, UserService};
use crate::utils::Checker;
use crate::{tt, AppState, AppString};
use axum::extract::State;

/// Registered User
#[utoipa::path(
    method(post),
    path = "/signup",
    security(
        (),
    ),
    request_body = UserInfo,
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::USER_TAG
)]
pub(crate) async fn user_signup(
    State(state): State<AppState>,
    SnJson(info): SnJson<UserInfo>,
) -> ApiResponseResult<AppString> {
    if info.password.len() < 8 {
        return Err(ApiError::User(tt!("messages.user.signup.password_format")));
    }
    UserService::signup(info, &state).await?;
    Ok(tt!("messages.user.signup.success").into())
}

/// Reset Password
#[utoipa::path(
    method(post),
    path = "/reset/password",
    security(
        (),
    ),
    request_body = UserResetPassword,
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::USER_TAG
)]
pub(crate) async fn reset_password(
    State(state): State<AppState>,
    status: UserStatus,
    SnJson(info): SnJson<UserResetPassword>,
) -> ApiResponseResult<String> {
    if !Checker::email(info.email.as_str()) {
        return Err(ApiError::User(tt!("messages.user.signup.email_format", email = info.email)));
    }
    if info.password.len() < 8 {
        return Err(ApiError::User(tt!("messages.user.signup.password_format")));
    }
    let s = UserService::reset_password(status, &info, &state).await?;
    Ok(s.into())
}

/// Send reset password verification code
#[utoipa::path(
    method(post),
    path = "/reset",
    security(
        (),
    ),
    request_body = UserReset,
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::USER_TAG
)]
pub(crate) async fn reset(
    lang: UserLang,
    State(state): State<AppState>,
    status: UserStatus,
    SnJson(info): SnJson<UserReset>,
) -> ApiResponseResult<AppString> {
    if !Checker::email(info.email.as_str()) {
        return Err(ApiError::User(tt!("messages.user.signup.email_format", email = info.email)));
    }

    let s = UserService::reset(status, &info, &state).await?;
    Ok(s.into())
}
