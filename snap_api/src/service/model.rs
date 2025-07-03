use crate::db::model::{DeviceModel, ModelMapper};
use crate::error::ApiResult;
use crate::service::user::CurrentUser;
use serde::{Deserialize, Serialize};
use snap_mq::config::db::DB;
use std::ops::Deref;
use uuid::Uuid;

pub(crate) struct ModelService;

#[derive(Serialize, Deserialize)]
pub(crate) struct ReqModel {
    name: String,
    model: ModelMapper,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ModelResp {
    id: Uuid,
    name: String,
    model: ModelMapper,
}
impl ModelService {
    pub(crate) async fn create(user: &CurrentUser, req: ReqModel, db: &DB) -> ApiResult {
        let _model = DeviceModel::insert(user.id, req.name.as_str(), req.model, db.deref()).await?;
        Ok(())
    }
    pub(crate) async fn list_model(user: &CurrentUser, db: &DB) -> ApiResult<Vec<ModelResp>> {
        let models = DeviceModel::query_by_user(user.id, db.deref()).await?;
        let models = models
            .into_iter()
            .map(|e| ModelResp {
                id: e.id,
                name: e.name,
                model: e.model.0,
            })
            .collect();
        Ok(models)
    }
}
