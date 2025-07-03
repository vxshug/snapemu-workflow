use crate::error::{ApiError, ApiResult};
use crate::service::device::DeviceService;
use crate::{get_lang, tt, CurrentUser};
use common_define::db::{
    Eui, Key, SnapDeviceActiveModel, SnapDeviceColumn, SnapDeviceEntity, SnapDeviceModel,
};
use common_define::product::{DeviceType, ProductType};
use common_define::Id;
use derive_new::new;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};
use serde::Deserialize;
use tracing::instrument;

pub(crate) struct SnapDeviceService;

#[derive(new)]
pub(crate) struct ReqSnap {
    name: String,
    description: String,
    eui: Eui,
    join_parameter: SnapJoinParameter,
}

#[derive(Deserialize)]
pub(crate) struct SnapJoinParameter {
    key: String,
}

impl SnapDeviceService {
    #[instrument(skip_all)]
    pub(crate) async fn create<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        req: ReqSnap,
        user: &CurrentUser,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult<SnapDeviceModel> {
        let device = DeviceService::register_device(
            user,
            req.eui,
            req.name.as_str(),
            req.description.as_str(),
            DeviceType::Snap,
            conn,
        )
        .await?;
        let key = Key::try_from(req.join_parameter.key.as_str())?;
        let snap = SnapDeviceActiveModel {
            id: Default::default(),
            device_id: ActiveValue::Set(device.id),
            eui: ActiveValue::Set(req.eui),
            key: ActiveValue::Set(key),
            product_type: ActiveValue::Set(ProductType::Monitor),
            battery: Default::default(),
            charge: ActiveValue::Set(false),
        };
        let snap = snap.insert(conn).await?;

        let info = device_info::snap::SnapDeviceInfo::new(
            device.id,
            key,
            None,
            0,
            None,
            device.script,
            None,
        );
        info.register(snap.eui, redis).await?;
        Ok(snap)
    }

    pub(crate) async fn delete<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        device: Id,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult {
        let model = SnapDeviceEntity::find()
            .filter(SnapDeviceColumn::DeviceId.eq(device))
            .one(conn)
            .await?;
        match model {
            Some(model) => {
                device_info::snap::SnapDeviceInfo::unregister(model.eui, redis).await?;
            }
            None => {
                return Err(crate::error::ApiError::Device {
                    device_id: device,
                    msg: tt!("messages.device.snap.eui_missing"),
                })
            }
        }
        Ok(())
    }

    pub(crate) async fn get_device<C: ConnectionTrait>(
        device_id: Id,
        conn: &C,
    ) -> ApiResult<SnapDeviceModel> {
        SnapDeviceEntity::find()
            .filter(SnapDeviceColumn::DeviceId.eq(device_id))
            .one(conn)
            .await?
            .ok_or(ApiError::Device {
                device_id,
                msg: tt!("messages.device.common.device_missing", device_id = device_id),
            })
    }

    #[instrument(skip(conn))]
    pub(crate) async fn get_all<C: ConnectionTrait>(
        id: Vec<Id>,
        conn: &C,
    ) -> ApiResult<Vec<SnapDeviceModel>> {
        if id.is_empty() {
            return Ok(Vec::new());
        }
        SnapDeviceEntity::find()
            .filter(SnapDeviceColumn::DeviceId.is_in(id))
            .all(conn)
            .await
            .map_err(Into::into)
    }

    fn device_last_key(device: Id) -> String {
        let lang = get_lang().as_static_str();
        format!("data:last:{}:{}", lang, device)
    }

    // pub(crate) async fn query_last(
    //     device: &[Vec<>],
    //     script_id: Option<Id>,
    //     user: &CurrentUser,
    //     conn: &mut DBConnection,
    // ) -> ApiResult<DataDeviceOneResponseWrap> {
    //     let key = Self::device_last_key(device);
    //     let lang = get_lang().as_static_str();
    //     let redis: RedisCli = Depend::get();
    //     let mut redis_conn = redis.get_multiplexed_tokio_connection().await?;
    //     let data_resp: Option<DataDeviceOneResponseWrap> = redis_conn.get(&key).await?;
    //     if let Some(data) = data_resp {
    //         if data.update > Time::data_expires() {
    //             return Ok(data)
    //         }
    //     }
    //
    //     let data: Option<DeviceData> = DBController::query(
    //         sqlx::query_as(DeviceData::SELECT_ONE_BY_DEVICE_ID)
    //             .bind(device)
    //             .fetch_optional(conn.as_mut()),
    //     ).await?;
    //     if data.is_none() {
    //         return Ok(DataDeviceOneResponseWrap {
    //             counts: 0,
    //             data: vec![],
    //             update: Time::now(),
    //         })
    //     }
    //     match data {
    //         None => {
    //             Ok(DataDeviceOneResponseWrap {
    //                 counts: 0,
    //                 data: vec![],
    //                 update: Time::now(),
    //             })
    //         }
    //         Some(data) => {
    //             let mut resp = vec![];
    //             match script_id {
    //                 None => {
    //                     let models: ModelMap = Depend::get();
    //                     for d in data.data.0 {
    //                         let data_name = models.get_entry(d.i, lang);
    //                         let data = DataDeviceOneResponse {
    //                             name: data_name.name.to_string(),
    //                             data_id: d.i,
    //                             unit: data_name.unit.to_string(),
    //                             data: TimeDate {
    //                                 time: data.time,
    //                                 data: d.v
    //                             }
    //                         };
    //                         resp.push(data)
    //                     };
    //                 }
    //                 Some(id) => {
    //                     let map = DecodeService::query_map(&[id], conn).await?;
    //                     for (index, d) in data.data.0.into_iter().enumerate() {
    //                         if let Some(map) = map.get(index) {
    //
    //                             let data = DataDeviceOneResponse {
    //                                 name: map.d_name.to_string(),
    //                                 data_id: d.i,
    //                                 unit: map.d_unit.to_string(),
    //                                 data: TimeDate {
    //                                     time: data.time,
    //                                     data: d.v
    //                                 }
    //                             };
    //                             resp.push(data)
    //                         }
    //                     };
    //                 }
    //             }
    //
    //             resp.sort_by(|pre, cur| pre.data_id.cmp(&cur.data_id));
    //             let resp = DataDeviceOneResponseWrap {
    //                 counts: resp.len() as i64,
    //                 data: resp,
    //                 update: Time::now(),
    //             };
    //
    //             redis_conn.set(&key, &resp).await?;
    //             Ok(resp)
    //         }
    //     }
    // }
}
