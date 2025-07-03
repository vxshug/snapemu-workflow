use crate::error::ApiError;
use crate::man::{RedisClient, RedisRecv};
use crate::{tt, AppState};
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Response, Sse};
use axum::routing::get;
use common_define::event::PlatformLog;
use tokio_stream::StreamExt;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().route("/", get(log))
}

#[derive(OpenApi)]
#[openapi(
    paths(log),
    tags((name = "log", description = "log api")),
)]
pub struct LogApi;

///
/// logs
#[utoipa::path(
    get,
    path = "/log",
    responses(
            (status = 0, description = "log", content_type = "text/event-stream"),
    )
)]
async fn log() -> Result<Response, ApiError> {
    let redis = RedisClient::get_client();
    let mut consumer = RedisRecv::new(
        redis.get_pubsub().await.map_err(|_| ApiError::User("redis connect error".into()))?,
    );
    consumer
        .subscribe(PlatformLog::TOPIC)
        .await
        .map_err(|_| ApiError::User("subscribe error".into()))?;
    let s = consumer.into_on_message().map(|event| {
        serde_json::from_slice::<serde_json::Value>(event.get_payload_bytes())
            .map_err(|_| ApiError::User(tt!("messages.device.log.stream_decode")))
            .and_then(|e| {
                Event::default()
                    .json_data(e)
                    .map_err(|_| ApiError::User(tt!("messages.device.log.stream_decode")))
            })
    });
    let mut response = Sse::new(s).keep_alive(KeepAlive::default()).into_response();
    response.headers_mut().insert("X-Accel-Buffering", "no".parse().unwrap()); // nginx
    Ok(response)
}
