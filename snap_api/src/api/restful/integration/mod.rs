use crate::AppState;
use axum::Router;
use utoipa_axum::router::OpenApiRouter;

mod mqtt;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().nest("/mqtt", mqtt::router())
}
