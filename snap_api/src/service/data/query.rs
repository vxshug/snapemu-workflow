use crate::error::ApiResult;
use crate::service::data::{DataService, DeviceData};
use crate::{get_lang, AppState, MODEL_MAP};
use std::collections::{BTreeMap, HashMap};
use std::ops::Sub;

use common_define::db::{DecodeScriptEntity, DeviceDataColumn, DeviceDataEntity, DevicesModel};
use common_define::decode::{LastDecodeData, Value};
use common_define::product::DeviceType;
use common_define::time::Timestamp;
use common_define::{last_device_data_key, Id};
use derive_new::new;
use redis::AsyncCommands;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::man::influxdb::InfluxQueryBuilder;

#[derive(Deserialize, Serialize, Clone, new)]
pub(crate) struct TimeDate {
    pub time: i64,
    pub data: Value,
}
#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct DataResponse {
    pub name: String,
    pub counts: i32,
    pub data_id: u32,
    pub unit: String,
    pub data: Vec<TimeDate>,
}

#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct DataDeviceOneResponse {
    pub(crate) name: String,
    pub(crate) data_id: u32,
    pub(crate) unit: String,
    pub(crate) data: TimeDate,
}
#[derive(Deserialize, Serialize, Clone, redis_macros::FromRedisValue, redis_macros::ToRedisArgs)]
pub(crate) struct DataDeviceOneResponseWrap {
    pub counts: i64,
    pub data: Vec<DataDeviceOneResponse>,
    pub update: Timestamp,
}

#[derive(Deserialize, Serialize, Clone, redis_macros::FromRedisValue, redis_macros::ToRedisArgs)]
pub(crate) struct DataResponseWrap {
    pub counts: i64,
    pub data: Vec<DataResponse>,
    pub update: Timestamp,
}

#[derive(Copy, Clone, Debug)]
pub enum DataDuration {
    Hour,
    Day,
    Week,
}



impl DataDuration {
    fn duration(&self) -> chrono::Duration {
        match self {
            DataDuration::Hour => chrono::Duration::hours(1),
            DataDuration::Day => chrono::Duration::days(1),
            DataDuration::Week => chrono::Duration::weeks(1),
        }
    }

    fn influx_range(&self) -> String {
        match self {
            DataDuration::Hour => String::from("|> range(start: -1h)"),
            DataDuration::Day => String::from("|> range(start: -1d)"),
            DataDuration::Week => String::from("|> range(start: -1w)"),
        }
    }
}

impl Sub<DataDuration> for Timestamp {
    type Output = Timestamp;

    fn sub(self, rhs: DataDuration) -> Self::Output {
        match rhs {
            DataDuration::Hour => self - chrono::Duration::hours(1),
            DataDuration::Day => self - chrono::Duration::days(1),
            DataDuration::Week => self - chrono::Duration::weeks(1),
        }
    }
}

fn get_device_name(device_name: String) -> String {
    match device_name.split_once(",") {
        None => {
            device_name
        }
        Some((name, _)) => {
            name.to_string()
        }
    }
}

impl DataService {
    // pub(crate) async fn query_device(
    //     user: &CurrentUser,
    //     device: Id,
    //     hour: i64,
    //     conn: &mut DBConnection,
    // ) -> ApiResult<DataResponseWrap> {
    //     let time = Utc::now() - Duration::hours(hour);
    //     let data_all: Vec<DeviceData> = DBController::query(
    //         sqlx::query_as(DeviceData::SELECT_BY_DEVICE_ID_AND_TIME)
    //             .bind(device)
    //             .bind(time)
    //             .fetch_all(conn.as_mut()),
    //     ).await?;
    //     let mut data_map: BTreeMap<u32, Vec<TimeDate>> = BTreeMap::new();
    //
    //
    //
    //     for data in data_all {
    //         for d in data.data.0 {
    //             if let Some(data_vec) = data_map.get_mut(&d.i) {
    //                 data_vec.push(TimeDate::new(data.time.clone(), d.v))
    //             } else {
    //                 data_map.insert(d.i, vec![TimeDate::new(data.time.clone(), d.v)]);
    //             }
    //         }
    //     };
    //
    //     let models: &ModelMap = GLOBAL_DEP.get_ref();
    //
    //
    //     let mut resp = vec![];
    //     for (data_id, device_data) in data_map {
    //         let data_name = models.get_entry(data_id as u32, user.lang.as_ref());
    //
    //
    //         let data = DataResponse {
    //             name: data_name.name.to_string(),
    //             counts: device_data.len() as i32,
    //             data_id,
    //             unit: data_name.unit.to_string(),
    //             data: device_data,
    //         };
    //         resp.push(data)
    //     };
    //     Ok(DataResponseWrap {
    //         counts: resp.len() as i64,
    //         data: resp
    //     })
    // }

    // pub(crate) async fn query_data(
    //     user: &CurrentUser,
    //     device: Id,
    //     data_id: Option<i32>,
    //     hour: i64,
    //     conn: &mut DBConnection,
    // ) -> ApiResult<DataResponseWrap> {
    //     let time = Utc::now() - Duration::hours(hour);
    //     let mut data_ids: HashSet<i32> = HashSet::new();
    //     let data_all: Vec<DeviceData> = match data_id {
    //         None => {
    //             let data: Vec<DeviceData> = DBController::query(
    //                 sqlx::query_as(DeviceData::SELECT_ALL_BY_DEVICE_ID_AND_TIME)
    //                     .bind(device)
    //                     .bind(time)
    //                     .fetch_all(conn.as_mut()),
    //             ).await?;
    //             for datum in &data {
    //                 data_ids.insert(datum.data_id);
    //             }
    //             data
    //         }
    //         Some(data_id) => {
    //             data_ids.insert(data_id);
    //             DBController::query(
    //                 sqlx::query_as(DeviceData::SELECT_ALL_BY_DEVICE_ID_AND_TIME_DATA_ID)
    //                     .bind(device)
    //                     .bind(data_id)
    //                     .bind(time)
    //                     .fetch_all(conn.as_mut()),
    //             ).await?
    //         }
    //     };
    //
    //     let models: &ModelMap = GLOBAL_DEP.get_ref();
    //     let mut data_map: BTreeMap<i32, DataResponse> = BTreeMap::new();
    //
    //     for data in data_all {
    //         match data_map.get_mut(&data.data_id) {
    //             Some(d) => {
    //                 d.counts += 1;
    //                 d.data.push(
    //                     TimeDate {
    //                         time: data.time,
    //                         data: data.data.0
    //                     }
    //                 )
    //             }
    //             None => {
    //                 let data_name = models.get_entry(data.data_id as u32, user.lang.as_ref());
    //                 let res=
    //                     DataResponse {
    //                         name: data_name.name.to_string(),
    //                         counts: 1,
    //                         data_id: data.data_id,
    //                         unit: data_name.unit.to_string(),
    //                         v_type: data.v_type,
    //                         data: vec![
    //                             TimeDate {
    //                                 time: data.time,
    //                                 data: data.data.0,
    //                             }
    //                         ],
    //                     };
    //                 data_map.insert(data.data_id, res);
    //             }
    //         }
    //     }
    //     let mut data: Vec<DataResponse> = data_map.into_values().collect();
    //     data.sort_by(|pre, cur| pre.data_id.cmp(&cur.data_id));
    //
    //     Ok(DataResponseWrap {
    //         counts: data.len() as i64,
    //         data,
    //     })
    // }

    fn device_duration_key(device: Id, data_duration: DataDuration) -> String {
        let lang = get_lang().as_static_str();
        match data_duration {
            DataDuration::Hour => {
                format!("data:hour:{}:{}", lang, device)
            }
            DataDuration::Day => {
                format!("data:day:{}:{}", lang, device)
            }
            DataDuration::Week => {
                format!("data:week:{}:{}", lang, device)
            }
        }
    }

    fn device_last_key(device: Id) -> String {
        let lang = get_lang().as_static_str();
        format!("data:last:{}:{}", lang, device)
    }
    pub(crate) async fn query_duration_data(
        device: Id,
        data_duration: DataDuration,
        state: &AppState,
    ) -> ApiResult<DataResponseWrap> {
        let influx_range = data_duration.influx_range();
        let filter = format!("|> filter(fn: (r) => r[\"_measurement\"] == \"{}\")", device);
        let builder = InfluxQueryBuilder::new_with_fns(vec![
            &influx_range,
            &filter,
        ]);

        let data = state.influx_client.query(builder).await?;

        let mut data_map: HashMap<i64, Vec<DeviceData>> = HashMap::new();

        for item in data {
            if let Some(v) = data_map.get_mut(&item.id) {
                v.push(item);
                continue
            }
            data_map.insert(item.id, vec![item]);
        }

        let mut data_response = Vec::with_capacity(data_map.len());

        for (_, data) in data_map {
            let mut data_name: Option<String> = None;
            let mut data_unit: Option<String> = None;
            let mut data_id = 0u32;
            let mut time_data = Vec::with_capacity(data.len());
            for item in data {
                data_id = item.id as u32;
                if data_name.is_none() {
                    data_name = Some(item.name);
                }
                if data_unit.is_none() {
                    data_unit = Some(item.unit);
                }
                time_data.push(TimeDate::new(item.time, item.value));
            }
            if data_unit.is_none() || data_name.is_none() {
                data_name = None;
                data_unit = None;
                continue;
            }
            let res = DataResponse {
                name: get_device_name(data_name.unwrap()),
                counts: time_data.len() as _,
                data_id,
                unit: data_unit.unwrap(),
                data: time_data,
            };
            data_response.push(res);
            data_name = None;
            data_unit = None;
        }

        data_response.sort_by(|pre, cur| pre.data_id.cmp(&cur.data_id));

        let resp = DataResponseWrap { counts: data_response.len() as i64, data: data_response, update: Timestamp::now() };
        Ok(resp)
    }

    pub(crate) async fn query_last(
        device: &DevicesModel,
        state: &AppState,
    ) -> ApiResult<DataDeviceOneResponseWrap> {
        if device.device_type == DeviceType::LoRaGate {
            return Ok(DataDeviceOneResponseWrap {
                counts: 0,
                data: vec![],
                update: Timestamp::now(),
            });
        }
        let filter = format!("|> filter(fn: (r) => r[\"_measurement\"] == \"{}\")", device.id);
        let builder = InfluxQueryBuilder::new_with_fns(vec![
            "|> range(start: -1w)",
            &filter,
            "|> last()"
        ]);

        let data = state.influx_client.query(builder).await?;

        let mut resp = Vec::with_capacity(data.len());

        for item in data {
            let data = DataDeviceOneResponse {
                name: get_device_name(item.name),
                data_id: item.id as _,
                unit: item.unit,
                data: TimeDate { time: item.time, data: item.value },
            };
            resp.push(data);
        }

        let script_id = device.script;


        resp.sort_by(|pre, cur| pre.data_id.cmp(&cur.data_id));
        let resp = DataDeviceOneResponseWrap {
            counts: resp.len() as i64,
            data: resp,
            update: Timestamp::now(),
        };

        Ok(resp)
    }
}
