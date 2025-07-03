use chrono::Utc;
use influxdb2::models::data_point::DataPointError;
use influxdb2::models::DataPoint;
use common_define::decode::DecodeData;
use common_define::Id;
use snap_model::ModelSource;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq, Eq)]
#[serde(transparent)]
pub struct DbDecodeData(pub Vec<DecodeData>);

impl From<crate::decode::DecodeData> for DbDecodeData {
    fn from(value: crate::decode::DecodeData) -> Self {
        Self(
            value
                .data
                .into_iter()
                .map(|it| common_define::decode::DecodeData::new(it.i as u32, it.v))
                .collect(),
        )
    }
}

#[derive(thiserror::Error, Debug)]
pub enum DataError {
    #[error("datapoint build error: {0}")]
    DataPoint(#[from] DataPointError),
    #[error("Time")]
    Time
}

impl DbDecodeData {
    pub fn influxdb_build<S: ModelSource>(self, device_id: Id, map: S) -> Result<Vec<DataPoint>, DataError> {
        let mut v = Vec::new();
        let timestamp = Utc::now().timestamp_nanos_opt().ok_or(DataError::Time)?;
        for item in self.0 {
            let data_map = map.get_data_name(item.i);
            let data = DataPoint::builder(device_id.to_string())
                .tag("id", item.i.to_string())
                .tag("name", data_map.name.format_tag())
                .tag("unit", data_map.unit.to_string())
                .field("value", item.v)
                .timestamp(timestamp)
                .build()?;
            v.push(data);
        }
        Ok(v)
    }
}


