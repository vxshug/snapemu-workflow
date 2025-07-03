use crate::error::{ApiError, ApiResponseResult};
use crate::service::user::{save_picture, Picture};
use crate::{tt, AppState};
use axum::extract::{Multipart, State};
use axum::routing::get;
use common_define::db::{SnapProductInfoActiveModel, SnapProductInfoEntity};
use common_define::time::Timestamp;
use common_define::Id;
use futures_util::FutureExt;
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait};
use serde::Serialize;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().route("/", get(get_all_product).post(post_product))
}

#[derive(Serialize)]
struct ProductInfo {
    page: u64,
    count: u64,
    product: Vec<ProductInfoItem>,
}

#[derive(Serialize)]
struct ProductInfoItem {
    id: Id,
    sku: String,
    name: String,
    image: String,
    description: String,
    create_time: Timestamp,
}

#[derive(OpenApi)]
#[openapi(
    paths(get_all_product,post_product),
    tags((name = "product", description = "Device Product Info control api")),

)]
pub struct ProductApi;

///
/// Get all product info
#[utoipa::path(
    get,
    path = "/product",
    responses(
            (status = 0, description = "group page"),
    )
)]
async fn get_all_product(State(state): State<AppState>) -> ApiResponseResult<ProductInfo> {
    let product: Vec<_> = SnapProductInfoEntity::find()
        .all(&state.db)
        .await?
        .into_iter()
        .map(|item| ProductInfoItem {
            image: item.image,
            id: item.id,
            sku: item.sku,
            name: item.name,
            description: item.description,
            create_time: item.create_time,
        })
        .collect();
    Ok(ProductInfo { page: 0, count: product.len() as _, product }.into())
}

#[derive(utoipa::ToSchema)]
struct UpProduct {
    sku: String,
    name: String,
    description: String,
    image: Vec<u8>,
}

///
///create product info
#[utoipa::path(
    post,
    path = "/product",
    request_body(content = inline(UpProduct), content_type = "multipart/form-data"),
    responses(
            (status = 0, description = "group page"),
    )
)]
async fn post_product(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> ApiResponseResult<ProductInfoItem> {
    let mut sku: Option<String> = None;
    let mut name: Option<String> = None;
    let mut describption: Option<String> = None;
    let mut product_image: Option<_> = None;

    while let Some(field) =
        multipart.next_field().await.map_err(|e| ApiError::User(e.to_string().into()))?
    {
        if let Some(field_name) = field.name() {
            match field_name {
                "sku" => sku = field.text().await.ok(),
                "name" => name = field.text().await.ok(),
                "description" => describption = field.text().await.ok(),
                "image" => product_image = field.bytes().await.ok(),
                el => return Err(ApiError::User(format!("unsupported field: {:?}", el).into())),
            }
        }
    }
    let sku = sku.ok_or(ApiError::User("sku not found".into()))?;
    let name = name.ok_or(ApiError::User("name not found".into()))?;
    let description = describption.ok_or(ApiError::User("description not found".into()))?;
    let product_image = product_image.ok_or(ApiError::User("image not found".into()))?;
    let format = image::guess_format(product_image.as_ref())
        .or(Err(ApiError::User(tt!("messages.user.picture.format"))))?;

    let suffix = match format {
        image::ImageFormat::Png => "png",
        image::ImageFormat::Jpeg => "jpeg",
        image::ImageFormat::WebP => "webp",
        _ => return Err(ApiError::User(tt!("messages.user.picture.format"))),
    };
    let now = Timestamp::now();
    let picture = Picture::new(suffix, format.to_mime_type(), product_image);
    let s = format!("product/{}-{}", sku, now);
    let product_url = save_picture(picture, s).await?;
    let product = SnapProductInfoActiveModel {
        id: Default::default(),
        sku: ActiveValue::Set(sku),
        name: ActiveValue::Set(name),
        description: ActiveValue::Set(description),
        image: ActiveValue::Set(product_url),
        create_time: ActiveValue::Set(now),
    };
    let s = product.insert(&state.db).await.map(|item| ProductInfoItem {
        id: item.id,
        sku: item.sku,
        name: item.name,
        image: item.image,
        description: item.description,
        create_time: item.create_time,
    })?;
    Ok(s.into())
}
