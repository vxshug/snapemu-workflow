use crate::api::SnPath;
use crate::error::ApiError;
use crate::man::NodeEventManager;
use crate::service::device::DeviceService;
use crate::{get_current_user, tt, AppState};
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Response, Sse};
use axum::Extension;
use common_define::Id;
use tokio_stream::StreamExt;
use tracing::warn;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(log))
}

/// View device logs
#[utoipa::path(
    method(get),
    path = "/{id}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn log(
    SnPath(device): SnPath<Id>,
    State(state): State<AppState>,
    Extension(mg): Extension<NodeEventManager>,
) -> Result<Response, ApiError> {
    let user = get_current_user();
    DeviceService::query_one(user.id, device, &state.db).await?;
    let event = mg.subscribe(device);
    let s = event.into_stream().map(|event| match event {
        Ok(e) => Event::default().json_data(e).map_err(|e| {
            warn!("{}", e);
            ApiError::User(tt!("messages.device.log.stream_decode"))
        }),
        Err(e) => {
            warn!("{}", e);

            Err(ApiError::User(tt!("messages.device.log.stream_err")))
        }
    });
    let mut response = Sse::new(s).keep_alive(KeepAlive::default()).into_response();
    response.headers_mut().insert("X-Accel-Buffering", "no".parse().unwrap()); // nginx
    Ok(response)
}
