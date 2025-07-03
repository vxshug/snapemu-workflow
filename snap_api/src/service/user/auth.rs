use crate::error::{ApiError, ApiResponseResult, ApiResult};
use crate::man::RedisClient;
use crate::service::user::{RedisToken, UserService};
use crate::{run_with_user, tt};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{async_trait, http};
use common_define::time::Timestamp;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::log::debug;
use tracing::{info, warn};

#[derive(Clone, PartialEq, Copy)]
pub enum UserLang {
    EN,
    ZH,
}
impl UserLang {
    const FALLBACK_LANG: &'static str = "en";
    const SECOND_LANG: &'static str = "zh";
}

impl Default for UserLang {
    fn default() -> Self {
        Self::EN
    }
}

impl UserLang {
    pub(crate) fn as_static_str(&self) -> &'static str {
        match self {
            UserLang::EN => "en",
            UserLang::ZH => "zh",
        }
    }
}

impl UserLang {
    pub(crate) fn form_str(s: Option<&str>) -> Self {
        match s {
            None => Self::EN,
            Some(s) => {
                if s.starts_with(Self::ZH.as_static_str()) {
                    Self::ZH
                } else {
                    Self::EN
                }
            }
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for UserLang {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(http::header::ACCEPT_LANGUAGE)
            .and_then(|header| header.to_str().ok());
        Ok(Self::form_str(auth_header))
    }
}

#[derive(Clone)]
pub(crate) struct AuthorizationToken(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for AuthorizationToken {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header =
            parts.headers.get(http::header::AUTHORIZATION).and_then(|header| header.to_str().ok());
        match auth_header {
            Some(token) => {
                let format = token.split_once("Bearer ");
                match format {
                    None => Err(ApiError::User("`Authorization` start with Bearer".into())),
                    Some((_, auth)) => {
                        info!("auth: {}", auth);
                        Ok(Self(auth.into()))
                    }
                }
            }
            None => Err(ApiError::User("Not Found Authorization".into())),
        }
    }
}

pub async fn status() -> ApiResponseResult<HashMap<String, String>> {
    let mut h = HashMap::new();
    {
        let f = GLOBAL_REQUEST.lock().unwrap();
        for (k, v) in f.iter() {
            let s = *v + chrono::Duration::hours(8);
            h.insert(k.to_string(), s.to_rfc3339());
        }
    }
    Ok(h.into())
}

pub(crate) async fn auth(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header =
        req.headers().get(http::header::AUTHORIZATION).and_then(|header| header.to_str().ok());

    let auth_header = if let Some(auth_header) = auth_header {
        auth_header
    } else {
        warn!("Not auth");
        return Ok(ApiError::User("Not Found Authorization".into()).into_response());
    };

    match authorize_current_user(auth_header).await {
        Ok(user) => {
            {
                let mut s = GLOBAL_REQUEST.lock().unwrap();
                match s.get_mut(user.name.as_str()) {
                    None => {
                        s.insert(user.name.clone(), Timestamp::now());
                    }
                    Some(u) => {
                        *u = Timestamp::now();
                    }
                }
            }
            debug!("request user id: {}, username: {}, path: {}", user.id, user.name, req.uri().path_and_query().map(|x| x.as_str()).unwrap_or_default());
            Ok(run_with_user(user.id, user.name, next.run(req)).await)
        }
        Err(e) => Ok(e.into_response()),
    }
}
static GLOBAL_REQUEST: Lazy<Mutex<HashMap<String, Timestamp>>> =
    Lazy::new(|| Mutex::new(Default::default()));

async fn authorize_current_user(auth_token: &str) -> ApiResult<RedisToken> {
    let format = auth_token.split_once("Bearer ");
    match format {
        None => Err(ApiError::Access(tt!("messages.user.login.auth_begin"))),
        Some((_, auth)) => {
            debug!("auth: {}", auth);
            let mut s = RedisClient::get_client().get().await?;
            Ok(UserService::token_login(auth, &mut s).await?)
        }
    }
}
