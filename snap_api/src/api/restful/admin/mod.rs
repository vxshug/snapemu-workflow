use crate::error::ApiResponse;
use crate::load::load_config;
use crate::AppState;
use axum::http::{Request, StatusCode};
use axum::middleware;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use common_define::time::Timestamp;
use common_define::Id;
use hmac::Hmac;
use jwt::VerifyWithKey;
use serde::{Deserialize, Serialize};
use sha2::digest::KeyInit;
use sha2::Sha256;
use std::future::Future;
use tracing::Instrument;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_axum::router::OpenApiRouter;
use uuid::Uuid;

mod config;
mod device;
mod group;
mod log;
mod login;
mod product;
mod user;

#[derive(OpenApi)]
#[openapi(
    tags(
            (name = "admin"),
            (name = "Snapemu Admin", description = "Snapemu Admin API")
    ),
    nest(
        (path = "/admin", api = login::UserApi),
        (path = "/admin", api = device::DeviceApi),
        (path = "/admin", api = user::UserApi),
        (path = "/admin", api = group::GroupApi),
        (path = "/admin", api = config::ConfigApi),
        (path = "/admin", api = log::LogApi),
        (path = "/admin", api = product::ProductApi),
    ),
    security(
        ("Authorization" = []),
    ),
    modifiers(&AdminSecurityAddon),
    components(schemas(

    ))
)]
pub struct AdminApi;

struct AdminSecurityAddon;

impl Modify for AdminSecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "Authorization",
                SecurityScheme::Http(
                    HttpBuilder::new().scheme(HttpAuthScheme::Bearer).bearer_format("JWT").build(),
                ),
            )
        }
    }
}

pub(crate) fn router() -> OpenApiRouter<AppState> {
    let config = load_config();
    let auth = OpenApiRouter::new()
        .nest("/device", device::router())
        .nest("/user", user::router())
        .nest("/group", group::router())
        .nest("/config", config::router())
        .nest("/log", log::router())
        .nest("/product", product::router())
        .layer(middleware::from_fn(auth));

    if config.api.openapi {
        OpenApiRouter::new()
            .nest("/login", login::router())
            // .merge( Scalar::with_url("/scalar", AdminApi::openapi()))
            .merge(auth)
    } else {
        OpenApiRouter::new().nest("/login", login::router()).merge(auth)
    }
}

tokio::task_local! {
    static ADMIN: AdminUser;
}

fn get_admin_user() -> AdminUser {
    ADMIN.with(|it| it.clone())
}

#[derive(Clone)]
struct AdminUser {
    id: Id,
    username: String,
}

#[derive(Deserialize, Serialize)]
struct JwtClaims {
    uid: Uuid,
    aid: Id,
    exp: u64,
    name: String,
}

async fn run_with_admin<F>(id: Id, username: String, f: F) -> F::Output
where
    F: Future,
{
    let span_user = username.clone();

    let user = AdminUser { id, username };
    ADMIN
        .scope(user, f)
        .instrument(tracing::debug_span!("admin", user_id = id.to_string(), username = span_user))
        .await
}

pub(crate) async fn auth(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    match req.headers().get(axum::http::header::AUTHORIZATION).and_then(|it| it.to_str().ok()) {
        None => Ok(ApiResponse::auth("not found authorization").into_response()),
        Some(auth) => match auth.split_once("Bearer ") {
            None => Ok(ApiResponse::auth("not found Bearer authorization").into_response()),
            Some((_, jwt_str)) => {
                let config = load_config();
                let jwt_key = config.jwt_key.as_str();
                let jwt_key: Hmac<Sha256> = Hmac::new_from_slice(jwt_key.as_bytes())
                    .map_err(|_| StatusCode::UNAUTHORIZED)?;
                let jwt: JwtClaims = match jwt_str.verify_with_key(&jwt_key) {
                    Ok(o) => o,
                    Err(e) => return Ok(ApiResponse::auth(e.to_string()).into_response()),
                };
                let now = Timestamp::now().timestamp_millis();
                if jwt.exp < now {
                    return Ok(ApiResponse::auth("Token expires").into_response());
                }
                Ok(run_with_admin(jwt.aid, jwt.name, next.run(req)).await)
            }
        },
    }
}
