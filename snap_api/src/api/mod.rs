use crate::error::{ApiResponse, ApiStatus};
use async_graphql::async_trait::async_trait;
use axum::extract::{FromRequest, FromRequestParts, Path};
use axum::http::request::Parts;
use axum::Json;
use serde::de::DeserializeOwned;

pub(crate) mod restful;

pub struct SnJson<T>(pub T);

pub struct SnPath<T>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for SnPath<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ApiResponse;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Path::from_request_parts(parts, state)
            .await
            .map(|t| Self(t.0))
            .map_err(|e| ApiResponse::new(ApiStatus::Parm, (), e.body_text(), ()))
    }
}

#[async_trait]
impl<T, S> FromRequest<S> for SnJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiResponse;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::from_request(req, state)
            .await
            .map(|t| Self(t.0))
            .map_err(|e| ApiResponse::new(ApiStatus::Parm, (), e.body_text(), ()))
    }
}
