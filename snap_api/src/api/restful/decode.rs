use crate::api::{SnJson, SnPath};
use crate::error::ApiResponseResult;
use crate::service::decode::{DecodeService, ScriptRequest};
use crate::{get_current_user, AppState};
use axum::extract::State;
use common_define::Id;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(new_script, list_script)).routes(routes!(delete_script))
}

/// Create JS script
#[utoipa::path(
    method(post),
    path = "",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DECODE_TAG
)]
async fn new_script(
    State(state): State<AppState>,
    SnJson(req): SnJson<ScriptRequest>,
) -> ApiResponseResult<ScriptRequest> {
    let user = get_current_user();
    let script = DecodeService::insert_script(&user, req, &state.db).await?;
    Ok(script.into())
}

/// Get all JS scripts
#[utoipa::path(
    method(get),
    path = "",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DECODE_TAG
)]
async fn list_script(State(state): State<AppState>) -> ApiResponseResult<Vec<ScriptRequest>> {
    let user = get_current_user();

    let s = DecodeService::list(&user, &state.db).await?;
    Ok(s.into())
}

/// Delete JS scripts
#[utoipa::path(
    method(delete),
    path = "/{id}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DECODE_TAG
)]
async fn delete_script(
    State(state): State<AppState>,
    SnPath(id): SnPath<Id>,
) -> ApiResponseResult<String> {
    let user = get_current_user();

    DecodeService::delete_script(&user, id, &state.db).await?;
    Ok(String::new().into())
}
