use crate::api::{SnJson, SnPath};
use crate::error::{ApiError, ApiResponseResult};
use crate::service::device::device::DeviceWithAuth;
use crate::service::device::DeviceService;
use crate::{get_current_user, AppState};
use axum::extract::State;
use base64::Engine;
use common_define::db::{SnapDownLinkActiveModel, SnapDownLinkColumn, SnapDownLinkEntity};
use common_define::event::{DeviceEvent, DownloadMessage};
use common_define::product::DeviceType;
use common_define::time::Timestamp;
use common_define::Id;
use redis::AsyncCommands;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, ModelTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(post_down))
        .routes(routes!(get_template, post_template))
        .routes(routes!(delete_template))
}

#[derive(Deserialize)]
struct DownData {
    port: Option<u8>,
    data: String,
}

#[derive(Deserialize, Serialize)]
struct DownTemplateItem {
    id: Id,
    name: String,
    data: String,
    port: i32,
}

#[derive(Deserialize, Serialize)]
struct DownTemplate {
    data: Vec<DownTemplateItem>,
}

#[derive(Deserialize, Serialize)]
struct DownTempleBody {
    name: String,
    data: String,
    port: i32,
}

/// Send data to the device
#[utoipa::path(
    method(post),
    path = "/{id}/down",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn post_down(
    State(state): State<AppState>,
    SnPath(id): SnPath<Id>,
    SnJson(data): SnJson<DownData>,
) -> ApiResponseResult {
    base64::engine::general_purpose::STANDARD
        .decode(data.data.as_bytes())
        .map_err(|e| ApiError::User("invalid data".into()))?;
    let user = get_current_user();
    let conn = &state.db;
    let DeviceWithAuth { auth, device } =
        DeviceService::query_one_with_auth(user.id, id, conn).await?;
    match device.device_type {
        DeviceType::Snap => {
            let event = DownloadMessage {
                eui: device.eui,
                port: data.port.unwrap_or(2),
                data: data.data,
            };
            let data = serde_json::to_string(&event)?;
            let mut conn = state.redis.get().await?;
            conn.publish(DeviceEvent::DOWN_TOPIC, data).await?;
        }
        DeviceType::LoRaNode => {
            let event = DownloadMessage {
                eui: device.eui,
                port: data.port.unwrap_or(2),
                data: data.data,
            };
            let data = serde_json::to_string(&event)?;
            let mut conn = state.redis.get().await?;
            conn.publish(DeviceEvent::DOWN_TOPIC, data).await?;
        }
        _ => {
            return Err(ApiError::User("unsupport device type".into()));
        }
    }
    Ok(().into())
}

/// Get the template for sending data
#[utoipa::path(
    method(get),
    path = "/{id}/template",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn get_template(
    State(state): State<AppState>,
    SnPath(id): SnPath<Id>,
) -> ApiResponseResult<DownTemplate> {
    let user = get_current_user();
    let DeviceWithAuth { auth, device } =
        DeviceService::query_one_with_auth(user.id, id, &state.db).await?;
    let templates = SnapDownLinkEntity::find()
        .filter(SnapDownLinkColumn::UserId.eq(user.id).and(SnapDownLinkColumn::DeviceId.eq(id)))
        .all(&state.db)
        .await?;
    let v: Vec<_> = templates
        .into_iter()
        .map(|link| DownTemplateItem {
            id: link.id,
            name: link.name,
            data: link.data,
            port: link.port,
        })
        .collect();
    Ok(DownTemplate { data: v }.into())
}

/// Create a template for sending data
#[utoipa::path(
    method(post),
    path = "/{id}/template",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn post_template(
    State(state): State<AppState>,
    SnPath(id): SnPath<Id>,
    SnJson(template): SnJson<DownTempleBody>,
) -> ApiResponseResult<DownTemplateItem> {
    let user = get_current_user();
    let DeviceWithAuth { auth, device } =
        DeviceService::query_one_with_auth(user.id, id, &state.db).await?;

    let model = SnapDownLinkActiveModel {
        id: Default::default(),
        device_id: ActiveValue::Set(id),
        user_id: ActiveValue::Set(user.id),
        name: ActiveValue::Set(template.name),
        data: ActiveValue::Set(template.data),
        order: ActiveValue::Set(0),
        port: ActiveValue::Set(template.port),
        create_time: ActiveValue::Set(Timestamp::now()),
    };
    let ok = model.insert(&state.db).await?;
    Ok(DownTemplateItem { id: ok.id, name: ok.name, data: ok.data, port: ok.port }.into())
}

/// Delete the template for sending data
#[utoipa::path(
    method(delete),
    path = "/{id}/template/{temp_id}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn delete_template(
    State(state): State<AppState>,
    SnPath((device_id, template)): SnPath<(Id, Id)>,
) -> ApiResponseResult {
    let user = get_current_user();
    let DeviceWithAuth { auth, device } =
        DeviceService::query_one_with_auth(user.id, device_id, &state.db).await?;
    let template = SnapDownLinkEntity::find_by_id(template)
        .one(&state.db)
        .await?
        .ok_or_else(|| ApiError::User("no template found".into()))?;
    if template.user_id != user.id {
        return Err(ApiError::User("invalid user".into()));
    }
    if template.device_id != device_id {
        return Err(ApiError::User("invalid device".into()));
    }
    template.delete(&state.db).await?;
    Ok(().into())
}
