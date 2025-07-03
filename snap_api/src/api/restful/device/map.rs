use crate::api::{SnJson, SnPath};
use crate::error::ApiResponseResult;
use crate::service::device::device::DeviceWithAuth;
use crate::service::device::DeviceService;
use crate::{get_current_user, AppState};
use axum::extract::State;
use common_define::db::{
    CustomDecodeMap, CustomMapItem, SnapDeviceDataNameActiveModel, SnapDeviceDataNameColumn,
    SnapDeviceDataNameEntity,
};
use common_define::decode::CustomDecodeDataType;
use common_define::time::Timestamp;
use common_define::Id;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(get_map, post_map))
}

#[derive(Deserialize, Serialize)]
struct DataMapItem {
    data_id: u32,
    data_name: String,
    data_unit: String,
    data_type: CustomDecodeDataType,
}

#[derive(Deserialize, Serialize)]
struct DataMap {
    name: String,
    map: Vec<DataMapItem>,
}

/// Get data name mapping
#[utoipa::path(
    method(get),
    path = "/{id}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn get_map(
    State(state): State<AppState>,
    SnPath(id): SnPath<Id>,
) -> ApiResponseResult<Option<DataMap>> {
    let user = get_current_user();
    let DeviceWithAuth { auth, device } =
        DeviceService::query_one_with_auth(user.id, id, &state.db).await?;
    let map = SnapDeviceDataNameEntity::find()
        .filter(
            SnapDeviceDataNameColumn::Owner
                .eq(user.id)
                .and(SnapDeviceDataNameColumn::DeviceId.eq(id)),
        )
        .one(&state.db)
        .await?;
    match map {
        Some(map) => {
            let v: Vec<_> = map
                .map
                .0
                .into_iter()
                .map(|it| DataMapItem {
                    data_id: it.id,
                    data_name: it.name,
                    data_unit: it.unit,
                    data_type: it.t,
                })
                .collect();
            Ok(Some(DataMap { name: map.name, map: v }).into())
        }
        None => Ok(None.into()),
    }
}

/// Create a data name map
#[utoipa::path(
    method(post),
    path = "/{id}",
    responses(
        (status = OK, description = "Success", body = str)
    ),
    tag = crate::DEVICE_TAG
)]
async fn post_map(
    State(state): State<AppState>,
    SnPath(id): SnPath<Id>,
    SnJson(map): SnJson<DataMap>,
) -> ApiResponseResult {
    let user = get_current_user();
    let DeviceWithAuth { auth, device } =
        DeviceService::query_one_with_auth(user.id, id, &state.db).await?;

    let custom_map = CustomDecodeMap(
        map.map
            .into_iter()
            .map(|it| CustomMapItem {
                id: it.data_id,
                name: it.data_name,
                unit: it.data_unit,
                t: it.data_type,
            })
            .collect(),
    );
    let model = SnapDeviceDataNameActiveModel {
        id: Default::default(),
        device_id: ActiveValue::Set(device.id),
        owner: ActiveValue::Set(user.id),
        name: ActiveValue::Set(map.name),
        map: ActiveValue::Set(custom_map),
        create_time: ActiveValue::Set(Timestamp::now()),
    };
    let ok = model.insert(&state.db).await?;
    Ok(().into())
}
