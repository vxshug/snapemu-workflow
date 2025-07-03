use crate::error::{ApiError, ApiResult};
use crate::service::device::DeviceService;
use crate::{tt, CurrentUser};
use common_define::db::{
    DeviceMqttActiveModel, DeviceMqttColumn, DeviceMqttEntity, DeviceMqttModel, Eui,
};
use common_define::product::DeviceType;
use common_define::Id;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};
use tracing::instrument;

pub(crate) struct MQTTService;

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct ReqMQTT {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) eui: Eui,
    pub(crate) username: String,
    pub(crate) password: String,
}

impl MQTTService {
    #[instrument(skip_all)]
    pub(crate) async fn create<C: ConnectionTrait>(
        req: ReqMQTT,
        user: &CurrentUser,
        conn: &C,
    ) -> ApiResult<DeviceMqttModel> {
        let device = DeviceService::register_device(
            user,
            req.eui,
            req.name.as_str(),
            req.description.as_str(),
            DeviceType::MQTT,
            conn,
        )
        .await?;
        let mqtt = DeviceMqttActiveModel {
            id: Default::default(),
            device_id: ActiveValue::Set(device.id),
            eui: ActiveValue::Set(req.eui),
            username: ActiveValue::Set(req.username),
            password: ActiveValue::Set(req.password),
        };
        let snap = mqtt.insert(conn).await?;

        Ok(snap)
    }

    #[instrument(skip_all)]
    pub(crate) async fn delete<C: ConnectionTrait>(device: Id, conn: &C) -> ApiResult {
        DeviceMqttEntity::delete_many()
            .filter(DeviceMqttColumn::DeviceId.eq(device))
            .exec(conn)
            .await?;
        Ok(())
    }

    #[instrument(skip_all)]
    pub(crate) async fn get<C: ConnectionTrait>(
        device_id: Id,
        conn: &C,
    ) -> ApiResult<DeviceMqttModel> {
        DeviceMqttEntity::find()
            .filter(DeviceMqttColumn::DeviceId.eq(device_id))
            .one(conn)
            .await?
            .ok_or(ApiError::Device {
                device_id,
                msg: tt!("messages.device.common.device_missing", device_id = device_id),
            })
    }
}
