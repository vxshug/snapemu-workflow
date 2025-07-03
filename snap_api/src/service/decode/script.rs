use crate::error::{ApiError, ApiResult};
use crate::service::decode::DecodeService;
use crate::{tt, CurrentUser};
use common_define::db::{
    CodeMapItem, DecodeMap as DbDecodeMap, DecodeScriptActiveModel, DecodeScriptColumn,
    DecodeScriptEntity, DevicesColumn, DevicesEntity,
};
use common_define::decode::{DecodeDataType, DecodeLang};
use common_define::time::Timestamp;
use common_define::Id;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel,
    ModelTrait, PaginatorTrait, QueryFilter,
};

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct DecodeMap {
    pub(crate) d_name: String,
    pub(crate) d_unit: String,
    pub(crate) d_type: DecodeDataType,
    pub(crate) d_id: u32,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct ScriptRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Id>,
    name: String,
    lang: DecodeLang,
    script: String,
    map: Vec<DecodeMap>,
}

#[derive(serde::Deserialize)]
pub(crate) struct ScriptResponse {
    name: String,
    lang: DecodeLang,
    script: String,
    map: Vec<DecodeMap>,
}

impl DecodeService {
    pub(crate) async fn update_script<C: ConnectionTrait>(
        user: &CurrentUser,
        req: ScriptRequest,
        conn: &C,
    ) -> ApiResult<ScriptRequest> {
        let script_id =
            req.id.ok_or_else(|| ApiError::User(tt!("messages.device.decode.id_missing")))?;
        let script = DecodeScriptEntity::find_by_id(script_id)
            .filter(DecodeScriptColumn::Owner.eq(user.id))
            .one(conn)
            .await?
            .ok_or(ApiError::User(tt!("messages.device.decode.not_found_script")))?;

        let map = DbDecodeMap(
            req.map
                .into_iter()
                .map(|it| CodeMapItem {
                    id: it.d_id,
                    name: it.d_name,
                    unit: it.d_unit,
                    t: it.d_type,
                })
                .collect(),
        );
        let mut model = script.into_active_model();
        model.script = ActiveValue::Set(req.script);
        model.name = ActiveValue::Set(req.name);
        model.map = ActiveValue::Set(map);
        model.modify_time = ActiveValue::Set(Timestamp::now());
        let item = model.update(conn).await?;
        let map = item
            .map
            .0
            .into_iter()
            .map(|m| DecodeMap { d_name: m.name, d_unit: m.unit, d_type: m.t, d_id: m.id })
            .collect();
        Ok(ScriptRequest {
            id: Some(item.id),
            name: item.name,
            lang: DecodeLang::JS,
            script: item.script,
            map,
        })
    }

    pub(crate) async fn delete_script<C: ConnectionTrait>(
        user: &CurrentUser,
        script: Id,
        conn: &C,
    ) -> ApiResult {
        let count =
            DevicesEntity::find().filter(DevicesColumn::Script.eq(script)).count(conn).await?;
        if count > 0 {
            return Err(ApiError::User(tt!("messages.device.decode.associated_device")));
        }
        let script = DecodeScriptEntity::find_by_id(script)
            .filter(DecodeScriptColumn::Owner.eq(user.id))
            .one(conn)
            .await?
            .ok_or(ApiError::User(tt!("messages.device.decode.not_found_script")))?;

        script.delete(conn).await?;

        Ok(())
    }

    pub(crate) async fn delete_user_script<C: ConnectionTrait>(user_id: Id, conn: &C) -> ApiResult {
        DecodeScriptEntity::delete_many()
            .filter(DecodeScriptColumn::Owner.eq(user_id))
            .exec(conn)
            .await?;
        Ok(())
    }
    pub(crate) async fn insert_script<C: ConnectionTrait>(
        user: &CurrentUser,
        script: ScriptRequest,
        conn: &C,
    ) -> ApiResult<ScriptRequest> {
        if script.id.is_some() {
            return Self::update_script(user, script, conn).await;
        }

        let map = DbDecodeMap(
            script
                .map
                .into_iter()
                .map(|it| CodeMapItem {
                    id: it.d_id,
                    name: it.d_name,
                    unit: it.d_unit,
                    t: it.d_type,
                })
                .collect(),
        );
        let now = Timestamp::now();
        let model = DecodeScriptActiveModel {
            id: Default::default(),
            script: ActiveValue::Set(script.script),
            lang: ActiveValue::Set(script.lang.as_ref().to_string()),
            owner: ActiveValue::Set(user.id),
            name: ActiveValue::Set(script.name),
            map: ActiveValue::Set(map),
            create_time: ActiveValue::Set(now),
            modify_time: ActiveValue::Set(now),
        };
        let item = model.insert(conn).await?;
        let map = item
            .map
            .0
            .into_iter()
            .map(|m| DecodeMap { d_name: m.name, d_unit: m.unit, d_type: m.t, d_id: m.id })
            .collect();
        Ok(ScriptRequest {
            id: Some(item.id),
            name: item.name,
            lang: DecodeLang::JS,
            script: item.script,
            map,
        })
    }
    pub(crate) async fn list<C: ConnectionTrait>(
        user: &CurrentUser,
        conn: &C,
    ) -> ApiResult<Vec<ScriptRequest>> {
        let scripts = DecodeScriptEntity::find()
            .filter(DecodeScriptColumn::Owner.eq(user.id))
            .all(conn)
            .await?;
        let scripts = scripts
            .into_iter()
            .map(|item| {
                let map = item
                    .map
                    .0
                    .into_iter()
                    .map(|m| DecodeMap { d_name: m.name, d_unit: m.unit, d_type: m.t, d_id: m.id })
                    .collect();
                ScriptRequest {
                    id: Some(item.id),
                    name: item.name,
                    lang: DecodeLang::JS,
                    script: item.script,
                    map,
                }
            })
            .collect();

        Ok(scripts)
    }
}
