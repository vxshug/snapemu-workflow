use crate::api::SnJson;
use crate::error::ApiResponseResult;
use crate::service::user::{Token, TokenService};
use crate::AppState;
use axum::extract::State;

#[derive(serde::Deserialize, serde::Serialize, Debug, utoipa::ToSchema)]
pub(crate) struct Refresh {
    refresh: String,
}

/// Refresh user token
#[utoipa::path(
    method(post),
    path = "/refresh",
    security(
        (),
    ),
    request_body = Refresh,
    responses(
        (status = OK, description = "Success", body = Token)
    ),
    tag = crate::USER_TAG
)]
pub(crate) async fn refresh(
    State(state): State<AppState>,
    SnJson(token): SnJson<Refresh>,
) -> ApiResponseResult<Token> {
    let token = TokenService::refresh_key(&token.refresh, &state).await?;
    Ok(token.into())
}
