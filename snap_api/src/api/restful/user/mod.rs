use crate::{service, AppState};
use axum::middleware;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

mod delete;
mod info;
pub(crate) mod login;
mod picture;
mod refresh;
mod signup;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    let picture = OpenApiRouter::new()
        .routes(routes!(picture::picture))
        .layer(middleware::from_fn(service::user::auth))
        .layer(axum::extract::DefaultBodyLimit::max(5 * 1024 * 1024));
    OpenApiRouter::new()
        .routes(routes!(info::info, info::get_info))
        .layer(middleware::from_fn(service::user::auth))
        .routes(routes!(login::user_login))
        .routes(routes!(signup::user_signup))
        .routes(routes!(signup::reset))
        .routes(routes!(signup::reset_password))
        .routes(routes!(refresh::refresh))
        .nest("/delete", delete::router())
        .merge(picture)
}
