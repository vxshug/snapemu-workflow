use crate::error::{ApiError, ApiResponse, ApiResponseResult, ApiStatus};
use crate::service::user::{Picture, UserService};
use crate::{get_current_user, tt, AppState, AppString};
use async_graphql::async_trait::async_trait;
use axum::extract::{FromRequest, FromRequestParts, Multipart, Request, State};
use axum_extra::headers::ContentLength;
use axum_extra::TypedHeader;
use futures_util::StreamExt;
use tracing::warn;

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct PictureUrl {
    url: String,
}

pub struct PictureFile(Multipart);

#[async_trait]
impl<S> FromRequest<S> for PictureFile
where
    S: Send + Sync,
{
    type Rejection = ApiResponse;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (mut parts, body) = req.into_parts();

        let TypedHeader(ContentLength(length)): TypedHeader<ContentLength> =
            TypedHeader::from_request_parts(&mut parts, state).await.map_err(|_| {
                ApiResponse::new(
                    ApiStatus::Parm,
                    Option::<()>::None,
                    "missing ContentLength header",
                    Option::<()>::None,
                )
            })?;
        if length > 5 * 1024 * 1024 {
            return Err(ApiResponse::new(
                ApiStatus::Parm,
                Option::<()>::None,
                "file too large",
                Option::<()>::None,
            ));
        }

        let req = Request::from_parts(parts, body);
        Multipart::from_request(req, state).await.map(PictureFile).map_err(|e| {
            ApiResponse::new(
                ApiStatus::Parm,
                Option::<()>::None,
                std::convert::Into::<AppString>::into(e.to_string()),
                Option::<()>::None,
            )
        })
    }
}

#[derive(utoipa::ToSchema)]
struct PictureForm {
    #[schema(content_media_type = "image/png")]
    img: Vec<u8>,
}

/// Modify user profile picture
#[utoipa::path(
    method(post),
    path = "/picture",
    security(
        (),
    ),
    request_body(content = inline(PictureForm), content_type = "multipart/form-data"),
    responses(
        (status = OK, description = "Success", body = PictureUrl)
    ),
    tag = crate::USER_TAG
)]
pub(crate) async fn picture(
    State(state): State<AppState>,
    PictureFile(mut multipart): PictureFile,
) -> ApiResponseResult<PictureUrl> {
    let user = get_current_user();
    let picture = multipart
        .next_field()
        .await
        .map_err(|e| {
            warn!("{}", e);
            ApiError::User(tt!("messages.user.picture.network"))
        })?
        .ok_or(ApiError::User(tt!("messages.user.picture.unfounded")))?;

    let data =
        picture.bytes().await.map_err(|_| ApiError::User(tt!("messages.user.picture.network")))?;

    let format = image::guess_format(data.as_ref())
        .or(Err(ApiError::User(tt!("messages.user.picture.format"))))?;

    let suffix = match format {
        image::ImageFormat::Png => "png",
        image::ImageFormat::Jpeg => "jpeg",
        image::ImageFormat::WebP => "webp",
        _ => return Err(ApiError::User(tt!("messages.user.picture.format"))),
    };
    let picture = Picture::new(suffix, format.to_mime_type(), data);
    let url = UserService::picture(&user, picture, &state.db).await?;
    Ok(PictureUrl { url }.into())
}
