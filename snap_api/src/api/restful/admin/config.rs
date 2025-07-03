use crate::api::SnJson;
use crate::error::ApiResponseResult;
use crate::AppState;
use axum::extract::State;
use axum::routing::get;
use common_define::db::{SnapConfigActiveModel, SnapConfigColumn, SnapConfigEntity};
use common_define::time::Timestamp;
use common_define::Id;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
};
use serde::Serialize;
use std::collections::HashMap;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

pub(crate) fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().route("/", get(get_all_config).put(put_config_info))
}

#[derive(Serialize)]
struct Config {
    pub config: Vec<ConfigItem>,
}

#[derive(Serialize)]
struct ConfigItem {
    pub id: Id,
    pub name: String,
    pub value: String,
    pub create_time: Timestamp,
}

#[derive(OpenApi)]
#[openapi(
    paths(get_all_config,put_config_info),
    tags((name = "config", description = "web control api")),
    components(schemas(
    ))
)]
pub struct ConfigApi;

///
/// Get all configurations
#[utoipa::path(
    get,
    path = "/config",
    responses(
            (status = 0, description = "user page"),
    )
)]
async fn get_all_config(State(state): State<AppState>) -> ApiResponseResult<Config> {
    let config = SnapConfigEntity::find().all(&state.db).await?;
    let config_item = config
        .into_iter()
        .map(|item| ConfigItem {
            id: item.id,
            name: item.name,
            value: item.value,
            create_time: item.create_time,
        })
        .collect();
    Ok(Config { config: config_item }.into())
}

///
/// Modify configuration
#[utoipa::path(
    put,
    path = "/config",
    request_body(content = HashMap<String, String>, description = "Pet to store the database", content_type = "application/json", example=json!({
        "app_version": "1.0.0"
    })),
    responses(
            (status = 0, description = "Modify configuration"),
    )
)]
async fn put_config_info(
    State(state): State<AppState>,
    SnJson(config): SnJson<HashMap<String, String>>,
) -> ApiResponseResult<Config> {
    let conn = &state.db;
    let mut v = vec![];
    for (name, value) in config {
        let it =
            SnapConfigEntity::find().filter(SnapConfigColumn::Name.eq(&name)).one(conn).await?;
        let r = match it {
            None => {
                let model = SnapConfigActiveModel {
                    id: Default::default(),
                    name: ActiveValue::Set(name),
                    value: ActiveValue::Set(value),
                    create_time: ActiveValue::Set(Timestamp::now()),
                };
                model.insert(conn).await?
            }
            Some(config_item) => {
                let mut model = config_item.into_active_model();
                model.value = ActiveValue::Set(value);
                model.update(conn).await?
            }
        };
        v.push(ConfigItem { id: r.id, name: r.name, value: r.value, create_time: r.create_time })
    }
    Ok(Config { config: v }.into())
}
