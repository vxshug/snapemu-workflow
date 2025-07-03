mod device;

use crate::AppState;
use axum::Router;

pub(crate) fn router() -> Router<AppState> {
    Router::new().nest("/device", device::router())
}
