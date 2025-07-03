use axum::extract::State;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tracing::instrument;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use common_define::db::{DeviceLoraGateColumn, DeviceLoraGateEntity};
use common_define::Id;
use crate::api::{SnJson, SnPath};
use crate::{get_current_user, AppState};
use crate::error::{ApiError, ApiResponseResult};

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes![get_config, put_config])
}

/// Get Gateway Config
///
#[utoipa::path(
    method(get),
    path = "/{id}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
#[instrument(skip(state))]
async fn get_config(
    State(state): State<AppState>,
    SnPath(id): SnPath<Id>,
) -> ApiResponseResult<snap_proto::manager::GwConfig> {
    let user = get_current_user();
    let gate = DeviceLoraGateEntity::find()
        .filter(DeviceLoraGateColumn::DeviceId.eq(id))
        .one(&state.db)
        .await?
        .ok_or_else(|| ApiError::User("GateWay does not exist".into()))?;
    let mut client = state.rpc.clone();
    let resp = client.config(tonic::Request::new(snap_proto::manager::GwConfigRequest {
        user_id: user.id.0,
        identity: Some(snap_proto::manager::DeviceIdentity {
            id: id.0,
            eui: gate.eui.0
        })
    })).await.map_err(|e| ApiError::User(e.message().to_string().into()))?;
    let conf = resp.into_inner();
    Ok(conf.into())
}

/// Modify Gateway Config
///
#[utoipa::path(
    method(put),
    path = "/{id}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
#[instrument(skip(state))]
async fn put_config(
    State(state): State<AppState>,
    SnPath(id): SnPath<Id>,
    SnJson(req): SnJson<snap_proto::manager::GwConfig>,
) -> ApiResponseResult<snap_proto::manager::GwConfig> {
    let user = get_current_user();
    let gate = DeviceLoraGateEntity::find()
        .filter(DeviceLoraGateColumn::DeviceId.eq(id))
        .one(&state.db)
        .await?
        .ok_or_else(|| ApiError::User("GateWay does not exist".into()))?;
    let mut client = state.rpc.clone();
    let resp = client.update_config(tonic::Request::new(snap_proto::manager::UpdateConfigRequest {
        user_id: user.id.0,
        identity: Some(snap_proto::manager::DeviceIdentity {
            id: id.0,
            eui: gate.eui.0
        }),
        config: Some(req)
    })).await.map_err(|e| ApiError::User(e.message().to_string().into()))?;
    let conf = resp.into_inner();
    Ok(conf.into())
}