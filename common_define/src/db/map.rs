use crate::decode::{CustomDecodeDataType, DecodeDataType};
use std::ops::Deref;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq, Eq)]
#[serde(transparent)]
pub struct DecodeMap(pub Vec<CodeMapItem>);
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq, Eq)]
#[serde(transparent)]
pub struct CustomDecodeMap(pub Vec<CustomMapItem>);

impl std::convert::From<CustomDecodeMap> for sea_orm::Value {
    fn from(source: CustomDecodeMap) -> Self {
        sea_orm::Value::Json(Some(Box::new(serde_json::to_value(source).unwrap_or_default())))
    }
}

impl sea_orm::TryGetable for CustomDecodeMap {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &sea_orm::QueryResult,
        idx: I,
    ) -> std::result::Result<Self, sea_orm::TryGetError> {
        <serde_json::Value as sea_orm::TryGetable>::try_get_by(res, idx).and_then(|v| {
            serde_json::from_value(v)
                .map_err(|e| sea_orm::TryGetError::DbErr(sea_orm::DbErr::Custom(e.to_string())))
        })
    }
}

impl sea_orm::sea_query::ValueType for CustomDecodeMap {
    fn try_from(v: sea_orm::Value) -> std::result::Result<Self, sea_orm::sea_query::ValueTypeErr> {
        <serde_json::Value as sea_orm::sea_query::ValueType>::try_from(v)
            .and_then(|v| serde_json::from_value(v).map_err(|_| sea_orm::sea_query::ValueTypeErr))
    }
    fn type_name() -> std::string::String {
        "CustomDecodeMap".to_owned()
    }
    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::Json
    }
    fn column_type() -> sea_orm::sea_query::ColumnType {
        sea_orm::prelude::ColumnType::Json
    }
}

impl Deref for DecodeMap {
    type Target = Vec<CodeMapItem>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::convert::From<DecodeMap> for sea_orm::Value {
    fn from(source: DecodeMap) -> Self {
        sea_orm::Value::Json(Some(Box::new(serde_json::to_value(source).unwrap_or_default())))
    }
}

impl sea_orm::TryGetable for DecodeMap {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &sea_orm::QueryResult,
        idx: I,
    ) -> std::result::Result<Self, sea_orm::TryGetError> {
        <serde_json::Value as sea_orm::TryGetable>::try_get_by(res, idx).and_then(|v| {
            serde_json::from_value(v)
                .map_err(|e| sea_orm::TryGetError::DbErr(sea_orm::DbErr::Custom(e.to_string())))
        })
    }
}

impl sea_orm::sea_query::ValueType for DecodeMap {
    fn try_from(v: sea_orm::Value) -> std::result::Result<Self, sea_orm::sea_query::ValueTypeErr> {
        <serde_json::Value as sea_orm::sea_query::ValueType>::try_from(v)
            .and_then(|v| serde_json::from_value(v).map_err(|_| sea_orm::sea_query::ValueTypeErr))
    }
    fn type_name() -> std::string::String {
        "DecodeMap".to_owned()
    }
    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::Json
    }
    fn column_type() -> sea_orm::sea_query::ColumnType {
        sea_orm::prelude::ColumnType::Json
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CodeMapItem {
    pub id: u32,
    pub name: String,
    pub unit: String,
    pub t: DecodeDataType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CustomMapItem {
    pub id: u32,
    pub name: String,
    pub unit: String,
    pub t: CustomDecodeDataType,
}
