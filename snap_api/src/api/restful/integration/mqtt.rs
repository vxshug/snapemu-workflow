use crate::api::{SnJson, SnPath};
use crate::error::{ApiError, ApiResponseResult};
use crate::service::integration::mqtt::{IntegrationMqttReq, MqttToken, MqttTokenResp};
use crate::service::integration::IntegrationService;
use crate::{get_current_user, AppState};
use axum::extract::State;
use sea_orm::TransactionTrait;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use common_define::Id;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(register,query, delete))
}

/// Create Integration
///
#[utoipa::path(
    method(post),
    path = "",
    request_body = IntegrationMqttReq,
    responses(
        (status = OK, description = "Success", body = MqttToken)
    ),
    tag = crate::INTEGRATION_TAG
)]
async fn register(
    State(state): State<AppState>,
    SnJson(req): SnJson<IntegrationMqttReq>,
) -> ApiResponseResult<MqttToken> {
    let token = state
        .db
        .transaction::<_, _, ApiError>(|ctx| {
            Box::pin(async move {
                let user = get_current_user();
                let token = IntegrationService::mqtt_register(&user, req, ctx).await?;
                Ok(token)
            })
        })
        .await?;

    Ok(token.into())
}


/// Get all Integration
///
#[utoipa::path(
    method(get),
    path = "",
    responses(
        (status = OK, description = "Success", body = MqttTokenResp)
    ),
    tag = crate::INTEGRATION_TAG
)]
async fn query(State(state): State<AppState>) -> ApiResponseResult<MqttTokenResp> {
    let user = get_current_user();

    let resp = IntegrationService::query_all(&user, &state.db).await?;
    Ok(resp.into())
}

/// Delete Integration
///
#[utoipa::path(
    method(delete),
    path = "/{id}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::INTEGRATION_TAG
)]
async fn delete(State(state): State<AppState>, SnPath(id): SnPath<Id>) -> ApiResponseResult<String> {
    let user = get_current_user();

    IntegrationService::delete(&user, id, &state.db).await?;
    Ok(String::new().into())
}
