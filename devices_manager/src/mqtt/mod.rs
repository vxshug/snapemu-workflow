use std::pin::Pin;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
use tracing::warn;
use common_define::db::{SnapIntegrationMqttColumn, SnapIntegrationMqttEntity};
use crate::GLOBAL_STATE;
use crate::man::Id;
use crate::protocol::mqtt::{MqttAuth, MqttAuthRequest};

pub mod down;

pub fn mqtt_auth_fn(mqtt_auth_request: MqttAuthRequest) -> Pin<Box<dyn std::future::Future<Output = Result<MqttAuth, crate::protocol::mqtt::LinkError>> + Send>> {
    Box::pin(async move {
        let tokens = SnapIntegrationMqttEntity::find()
            .filter(SnapIntegrationMqttColumn::Username.eq(&mqtt_auth_request.username))
            .all(&GLOBAL_STATE.db)
            .await.map_err(|_| crate::protocol::mqtt::LinkError::Db)?;

        for token in tokens {
            if token.password == mqtt_auth_request.password {
                return Ok(MqttAuth::new(token.user_id, token.username))
            }
        }
        Err(crate::protocol::mqtt::LinkError::InvalidAuth)
    })
}