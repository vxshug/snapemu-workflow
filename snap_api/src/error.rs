use crate::AppString;
use axum::response::{IntoResponse, Response};
use axum::Json;
use common_define::Id;
use num_enum::IntoPrimitive;
use redis::ErrorKind;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};
use crate::man::influxdb::InfluxError;

pub type ApiResult<T = ()> = Result<T, ApiError>;
pub(crate) type ApiResponseResult<T = ()> = Result<ApiResponse<T>, ApiError>;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Access: {0}")]
    Access(AppString),
    #[error("Refresh: {0}")]
    Refresh(AppString),
    #[error("Auth: {0}")]
    Auth(AppString),
    #[error("User: {0}")]
    User(AppString),
    #[error("Device: device_id={device_id}, msg={msg}")]
    Device { device_id: Id, msg: AppString },
    #[error("Server: case={case}, msg={msg}")]
    Server { case: &'static str, msg: AppString },
    #[error("Restart: case={case}, msg={msg}")]
    Restart { case: &'static str, msg: AppString },
}

impl ApiError {
    pub(crate) fn response(self) -> ApiResponse {
        match self {
            ApiError::Access(s) => ApiResponse::new(ApiStatus::AccessToken, (), s, ()),
            ApiError::Refresh(s) => ApiResponse::new(ApiStatus::RefreshToken, (), s, ()),
            ApiError::Auth(s) => ApiResponse::new(ApiStatus::RefreshToken, (), s, ()),
            ApiError::User(s) => ApiResponse::new(ApiStatus::User, (), s, ()),
            ApiError::Device { device_id, msg } => {
                warn!(device_id = device_id.to_string(), error = "device", "{}", msg);
                ApiResponse::new(ApiStatus::Device, (), "API server internal error", ())
            }
            ApiError::Server { case: source, msg } => {
                warn!(error = "server", source = source, "{}", msg);
                ApiResponse::new(ApiStatus::Db, (), "API server internal error", ())
            }
            ApiError::Restart { case: source, msg } => {
                error!(error = "restart", source = source, "{}", msg);
                ApiResponse::new(ApiStatus::Db, (), "API server internal error", ())
            }
        }
    }
}

impl From<common_define::db::DbErr> for ApiError {
    fn from(value: common_define::db::DbErr) -> Self {
        Self::Server { msg: value.to_string().into(), case: "db type" }
    }
}

impl From<deadpool::managed::PoolError<deadpool_redis::redis::RedisError>> for ApiError {
    fn from(value: deadpool::managed::PoolError<deadpool_redis::redis::RedisError>) -> Self {
        Self::Server { case: "redis pool", msg: value.to_string().into() }
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(value: reqwest::Error) -> Self {
        Self::Server { case: "reqwest", msg: value.to_string().into() }
    }
}

impl From<InfluxError> for ApiError {
    fn from(value: InfluxError) -> Self {
        error!("influxdb error: {}", value);
        Self::Server { case: "InfluxError", msg: "InfluxError".into() }
    }
}

impl From<redis::RedisError> for ApiError {
    fn from(value: redis::RedisError) -> Self {
        match value.kind() {
            ErrorKind::ResponseError
            | ErrorKind::TypeError
            | ErrorKind::BusyLoadingError
            | ErrorKind::NoScriptError
            | ErrorKind::Moved
            | ErrorKind::Ask
            | ErrorKind::TryAgain => {
                warn!("redis throw error: {}", value);
                Self::Server { case: "redis", msg: value.to_string().into() }
            }
            _ => Self::Restart { case: "restart", msg: value.to_string().into() },
        }
    }
}

impl From<sea_orm::DbErr> for ApiError {
    fn from(value: sea_orm::DbErr) -> Self {
        Self::Server { case: "sea_orm", msg: value.to_string().into() }
    }
}

impl From<sea_orm::TransactionError<ApiError>> for ApiError {
    fn from(value: sea_orm::TransactionError<ApiError>) -> Self {
        match value {
            sea_orm::TransactionError::Transaction(e) => {
                warn!("TransactionError: Transaction error: {}", e);
                e
            }
            sea_orm::TransactionError::Connection(e) => {
                warn!("TransactionError: connection error: {}", e);
                e.into()
            }
        }
    }
}
impl From<async_graphql::Error> for ApiError {
    fn from(value: async_graphql::Error) -> Self {
        Self::Server { case: "graphql", msg: value.message.into() }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(value: serde_json::Error) -> Self {
        Self::Server { case: "serde_json", msg: value.to_string().into() }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        self.response().into_response()
    }
}

/// The status code returned by the request
/// - Ok(0): Request successful
/// - Auth(1): Authentication request failed
/// - User(2): Authentication request failed
/// - Parm(3): Authentication request failed
/// - AccessToken(4): Authentication request failed
/// - RefreshToken(5): Authentication request failed
/// - Db(6): Authentication request failed
/// - Device(7): Authentication request failed
#[derive(IntoPrimitive, Serialize, Deserialize, Copy, Clone, utoipa::ToSchema)]
#[repr(u8)]
#[serde(into = "u8")]
pub enum ApiStatus {
    Ok,
    Auth,
    User,
    Parm,
    AccessToken,
    RefreshToken,
    Db,
    Device,
}

/// restful API Response
#[derive(Serialize, Deserialize, utoipa::ToSchema)]

pub struct ApiResponse<T = ()> {
    /// code is the returned state
    #[schema(value_type = i32, example = 0)]
    pub(crate) code: ApiStatus,
    /// notify Indicates a notification on the server
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = String, example = "The app has a new version")]
    pub(crate) notify: Option<AppString>,
    /// If the status code is not ok, then it is an error message,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = String, example = "This is a mistake")]
    pub(crate) message: Option<AppString>,
    /// The data returned by the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<T>,
}

pub(crate) trait IntoAppString {
    fn into_app_string(self) -> Option<AppString>;
}

impl IntoAppString for () {
    fn into_app_string(self) -> Option<AppString> {
        None
    }
}
impl IntoAppString for &'static str {
    fn into_app_string(self) -> Option<AppString> {
        Some(self.into())
    }
}

impl IntoAppString for String {
    fn into_app_string(self) -> Option<AppString> {
        Some(self.into())
    }
}

impl IntoAppString for AppString {
    fn into_app_string(self) -> Option<AppString> {
        Some(self)
    }
}

impl IntoAppString for Option<AppString> {
    fn into_app_string(self) -> Option<AppString> {
        self
    }
}
impl IntoAppString for Option<()> {
    fn into_app_string(self) -> Option<AppString> {
        None
    }
}

impl IntoAppString for Option<String> {
    fn into_app_string(self) -> Option<AppString> {
        self.map(Into::into)
    }
}

impl IntoAppString for Option<&'static str> {
    fn into_app_string(self) -> Option<AppString> {
        self.map(Into::into)
    }
}

impl<T> ApiResponse<T> {
    pub(crate) fn new<M: IntoAppString, N: IntoAppString>(
        code: ApiStatus,
        data: impl Into<Option<T>>,
        message: M,
        notify: N,
    ) -> Self {
        Self {
            code,
            notify: notify.into_app_string(),
            message: message.into_app_string(),
            data: data.into(),
        }
    }
}

impl ApiResponse {
    pub(crate) fn auth<M: IntoAppString>(message: M) -> Self {
        Self { code: ApiStatus::Auth, notify: None, message: message.into_app_string(), data: None }
    }
}

impl<T: Serialize> From<T> for ApiResponse<T> {
    fn from(value: T) -> Self {
        ApiResponse::new(ApiStatus::Ok, value, (), ())
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        Json::into_response(Json(self))
    }
}
