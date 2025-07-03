use chrono::Utc;
use influxdb2::models::data_point::DataPointError;
use influxdb2::models::DataPoint;
use crate::decode::DecodeData;
use crate::Id;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq, Eq)]
#[serde(transparent)]
pub struct DbDecodeData(pub Vec<DecodeData>);

#[derive(thiserror::Error, Debug)]
pub enum DataError {
    #[error("datapoint build error: {0}")]
    DataPoint(#[from] DataPointError)
}

impl From<DbDecodeData> for sea_orm::Value {
    fn from(source: DbDecodeData) -> Self {
        sea_orm::Value::Json(Some(Box::new(serde_json::to_value(source).unwrap_or_default())))
    }
}

impl sea_orm::TryGetable for DbDecodeData {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &sea_orm::QueryResult,
        idx: I,
    ) -> Result<Self, sea_orm::TryGetError> {
        <serde_json::Value as sea_orm::TryGetable>::try_get_by(res, idx).and_then(|v| {
            serde_json::from_value(v)
                .map_err(|e| sea_orm::TryGetError::DbErr(sea_orm::DbErr::Custom(e.to_string())))
        })
    }
}

impl sea_orm::sea_query::ValueType for DbDecodeData {
    fn try_from(v: sea_orm::Value) -> Result<Self, sea_orm::sea_query::ValueTypeErr> {
        <serde_json::Value as sea_orm::sea_query::ValueType>::try_from(v)
            .and_then(|v| serde_json::from_value(v).map_err(|_| sea_orm::sea_query::ValueTypeErr))
    }
    fn type_name() -> String {
        "DbDecodeData".to_owned()
    }
    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::Json
    }
    fn column_type() -> sea_orm::sea_query::ColumnType {
        sea_orm::prelude::ColumnType::Json
    }
}
