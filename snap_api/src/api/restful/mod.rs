use crate::{service, AppState};

use crate::api::restful::device::register_query;
use axum::middleware;
use axum::routing::{get, post};
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::Modify;
use utoipa_axum::router::OpenApiRouter;

mod admin;
mod app;
mod contact;
pub(crate) mod data;
mod decode;
pub(crate) mod device;
mod integration;
mod show;
pub(crate) mod user;
mod verify;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    let api = OpenApiRouter::new()
        .nest("/data", data::router())
        .nest("/integration", integration::router())
        .nest("/device", device::router())
        .nest("/decode", decode::router())
        // .nest("/show", show::router())
        .layer(middleware::from_fn(service::user::auth));

    OpenApiRouter::new()
        .route("/contact", post(contact::contact_us))
        .route("/app/version", get(app::version))
        .route("/device/query/register", post(register_query))
        // .nest("/verify", verify::router())
        .nest("/admin", admin::router())
        .nest("/user", user::router())
        .merge(api)
}

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "Authorization",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("Authorization"))),
            )
        }
    }
}
