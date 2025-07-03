use crate::error::{ApiError, ApiResult};
use crate::service::integration::IntegrationService;
use crate::{tt, CurrentUser};
use common_define::db::{
    SnapIntegrationMqttActiveModel, SnapIntegrationMqttColumn, SnapIntegrationMqttEntity,
};
use common_define::time::Timestamp;
use common_define::Id;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, ModelTrait,
    QueryFilter,
};
use common_define::product::MqttType;

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct IntegrationMqttReq {
    #[schema(example = "gateway")]
    name: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct MqttToken {
    /// Integration Id
    #[schema(example = "1")]
    id: Id,
    /// Integration name
    #[schema(example = "gateway")]
    name: String,
    /// Integration enable
    #[schema(example = "true")]
    enable: bool,
    /// username
    #[schema(example = "username")]
    username: String,
    /// password
    #[schema(example = "password")]
    password: String,
    /// create time
    #[serde(skip)]
    create_time: Timestamp,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct MqttTokenResp {
    /// count
    #[schema(example = "1")]
    count: usize,
    tokens: Vec<MqttToken>,
}

impl IntegrationService {
    pub async fn mqtt_register<C: ConnectionTrait>(
        user: &CurrentUser,
        device: IntegrationMqttReq,
        conn: &C,
    ) -> ApiResult<MqttToken> {
        let password = uuid::Uuid::new_v4().simple().to_string();
        loop {
            let entity = SnapIntegrationMqttEntity::find()
                .filter(SnapIntegrationMqttColumn::Password.eq(password.as_str()))
                .one(conn).await?;
            if entity.is_none() {
                break;
            }
        }

        let now = Timestamp::now();

        let model = SnapIntegrationMqttActiveModel {
            id: Default::default(),
            name: ActiveValue::Set(device.name),
            mqtt_type: ActiveValue::Set(MqttType::Integration),
            user_id: ActiveValue::Set(user.id),
            username: ActiveValue::Set(user.username.into()),
            password: ActiveValue::Set(password.into()),
            enable: ActiveValue::Set(true),
            create_time: ActiveValue::Set(now),
        };
        let model = model.insert(conn).await?;

        Ok(MqttToken {
            id: model.id,
            name: model.name,
            enable: model.enable,
            username: model.username,
            password: model.password,
            create_time: now,
        })
    }

    pub async fn query_all<C: ConnectionTrait>(
        user: &CurrentUser,
        conn: &C,
    ) -> ApiResult<MqttTokenResp> {
        let tokens = SnapIntegrationMqttEntity::find()
            .filter(SnapIntegrationMqttColumn::UserId.eq(user.id).and(SnapIntegrationMqttColumn::MqttType.eq(MqttType::Integration)))
            .all(conn)
            .await?;
        let tokens: Vec<MqttToken> = tokens
            .into_iter()
            .map(|item| MqttToken {
                id: item.id,
                name: item.name,
                enable: item.enable,
                username: item.username,
                password: item.password,
                create_time: item.create_time,
            })
            .collect();
        Ok(MqttTokenResp { count: tokens.len(), tokens })
    }

    pub async fn delete<C: ConnectionTrait>(user: &CurrentUser, id: Id, conn: &C) -> ApiResult {
        let token = SnapIntegrationMqttEntity::find_by_id(id).one(conn).await?;
        if let Some(token) = token {
            if token.user_id == user.id {
                token.delete(conn).await?;
            }
        }
        Ok(())
    }
}
