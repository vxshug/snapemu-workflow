use crate::sea_string_type;
use std::str::FromStr;

#[derive(
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    Copy,
    Clone,
    Debug,
    strum::AsRefStr,
    strum::EnumString,
    Eq
)]
pub enum DeviceType {
    LoRaNode,
    LoRaGate,
    MQTT,
    Snap,
}

impl std::convert::From<DeviceType> for sea_orm::Value {
    fn from(source: DeviceType) -> Self {
        source.as_ref().into()
    }
}
impl sea_orm::TryGetable for DeviceType {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &sea_orm::QueryResult,
        idx: I,
    ) -> std::result::Result<Self, sea_orm::TryGetError> {
        <String as sea_orm::TryGetable>::try_get_by(res, idx).and_then(|v| {
            DeviceType::from_str(&v)
                .map_err(|e| sea_orm::TryGetError::DbErr(sea_orm::DbErr::Custom(e.to_string())))
        })
    }
}
impl sea_orm::sea_query::ValueType for DeviceType {
    fn try_from(v: sea_orm::Value) -> std::result::Result<Self, sea_orm::sea_query::ValueTypeErr> {
        <String as sea_orm::sea_query::ValueType>::try_from(v)
            .and_then(|a| DeviceType::from_str(&a).map_err(|_| sea_orm::sea_query::ValueTypeErr))
    }
    fn type_name() -> std::string::String {
        "DeviceType".to_owned()
    }
    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::String
    }
    fn column_type() -> sea_orm::sea_query::ColumnType {
        sea_orm::prelude::ColumnType::Text
    }
}

#[derive(
    serde::Deserialize,
    serde::Serialize,
    redis_macros::FromRedisValue,
    redis_macros::ToRedisArgs,
    Copy,
    Clone,
    Debug,
    strum::AsRefStr,
    strum::EnumString,
    Eq,
    PartialEq
)]
pub enum ProductType {
    Custom,
    Monitor,
    Controller,
    Gate,
}
sea_string_type!(ProductType);

#[derive(
    serde::Deserialize,
    serde::Serialize,
    redis_macros::FromRedisValue,
    redis_macros::ToRedisArgs,
    Copy,
    Clone,
    Debug,
    strum::AsRefStr,
    strum::EnumString,
    Eq,
    PartialEq
)]
pub enum MqttType {
    Snap,
    Gateway,
    Integration
}
sea_string_type!(MqttType);

#[derive(
    Copy,
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    strum::AsRefStr,
    strum::EnumString,
    Eq,
    PartialEq
)]
pub enum ShareType {
    Group,
    User,
}

impl std::convert::From<ShareType> for sea_orm::Value {
    fn from(source: ShareType) -> Self {
        source.as_ref().into()
    }
}
impl sea_orm::TryGetable for ShareType {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &sea_orm::QueryResult,
        idx: I,
    ) -> std::result::Result<Self, sea_orm::TryGetError> {
        <String as sea_orm::TryGetable>::try_get_by(res, idx).and_then(|v| {
            ShareType::from_str(&v)
                .map_err(|e| sea_orm::TryGetError::DbErr(sea_orm::DbErr::Custom(e.to_string())))
        })
    }
}
impl sea_orm::sea_query::ValueType for ShareType {
    fn try_from(v: sea_orm::Value) -> std::result::Result<Self, sea_orm::sea_query::ValueTypeErr> {
        <String as sea_orm::sea_query::ValueType>::try_from(v)
            .and_then(|a| ShareType::from_str(&a).map_err(|_| sea_orm::sea_query::ValueTypeErr))
    }
    fn type_name() -> std::string::String {
        "ShareType".to_owned()
    }
    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::String
    }
    fn column_type() -> sea_orm::sea_query::ColumnType {
        sea_orm::prelude::ColumnType::Text
    }
}
