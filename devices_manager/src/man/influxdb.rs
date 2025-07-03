use chrono::Utc;
use derive_new::new;
use influxdb2::models::data_point::DataPointError;
use influxdb2::models::DataPoint;
use influxdb2::RequestError;
use common_define::Id;
use snap_model::ModelSource;
use crate::db::{DataError, DbDecodeData};
use crate::decode::DecodeData;

#[derive(Debug, thiserror::Error)]
pub enum InfluxError {
    #[error("RequestError {0}")]
    RequestError(#[from] RequestError),
    #[error("DataError {0}")]
    DataError(#[from] DataError),
    #[error("DataError {0}")]
    DataPointError(#[from] DataPointError)
}

#[derive(new)]
pub struct InfluxDbClient {
    bucket: String,
    client: influxdb2::Client
}

impl InfluxDbClient {

    pub async fn write_js(&self, data: DecodeData, device_id: Id) -> Result<(), InfluxError> {
        let timestamp = Utc::now().timestamp_nanos_opt().ok_or(DataError::Time)?;
        let mut v = Vec::new();
        for item in data.data {
            let data = DataPoint::builder(device_id.to_string())
                .tag("id", item.i.to_string())
                .tag("name", item.name)
                .tag("unit", item.unit)
                .field("value", item.v)
                .timestamp(timestamp)
                .build()?;
            v.push(data);
        }
        Ok(self.client.write(&self.bucket, tokio_stream::iter(v)).await?)
    }
}

