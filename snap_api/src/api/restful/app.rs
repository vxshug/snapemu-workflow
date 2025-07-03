use crate::error::ApiResponseResult;
use crate::AppState;
use axum::extract::State;
use common_define::db::{SnapConfigActiveModel, SnapConfigColumn, SnapConfigEntity};
use common_define::time::Timestamp;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;

#[derive(Serialize)]
pub struct Version {
    version: String,
}

pub async fn version(State(state): State<AppState>) -> ApiResponseResult<Version> {
    let version = match SnapConfigEntity::find()
        .filter(SnapConfigColumn::Name.eq("app_version"))
        .one(&state.db)
        .await?
    {
        Some(version) => version,
        None => {
            let mut model = <SnapConfigActiveModel as ActiveModelTrait>::default();
            model.name = ActiveValue::Set("app_version".to_string());
            model.value = ActiveValue::Set("1.0.0".to_string());
            model.create_time = ActiveValue::Set(Timestamp::now());
            model.insert(&state.db).await?
        }
    };

    let version = Version { version: version.value };
    Ok(version.into())
}
