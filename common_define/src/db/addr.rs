use crate::db::DbErr;
use derive_more::Into;
use redis::{FromRedisValue, RedisResult, RedisWrite, ToRedisArgs, Value};
use serde::de::Error;
use serde::{Deserializer, Serializer};
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

#[derive(Into, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct LoRaAddr(pub u32);

impl Debug for LoRaAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl LoRaAddr {
    pub fn new(k: u32) -> Self {
        Self(k)
    }
    pub fn random() -> Self {
        Self(rand::random())
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

impl serde::Serialize for LoRaAddr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}

impl<'de> serde::Deserialize<'de> for LoRaAddr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <&str as serde::Deserialize>::deserialize(deserializer)?;
        let id: LoRaAddr = s.parse().map_err(Error::custom)?;
        Ok(id)
    }
}

impl FromStr for LoRaAddr {
    type Err = DbErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        LoRaAddr::try_from(s)
    }
}

impl Display for LoRaAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode_upper(self.0.to_be_bytes()))
    }
}

impl From<[u8; 4]> for LoRaAddr {
    fn from(value: [u8; 4]) -> Self {
        Self(u32::from_le_bytes(value))
    }
}

impl TryFrom<&str> for LoRaAddr {
    type Error = DbErr;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 8 {
            return Err(DbErr::Len("addr most 8 byte, found ".to_string()));
        }
        let mut b = [0; 4];
        hex::decode_to_slice(value, &mut b).map_err(|_| DbErr::Parse)?;

        Ok(Self(u32::from_be_bytes(b)))
    }
}

impl ToRedisArgs for LoRaAddr {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        self.to_string().write_redis_args(out)
    }
}

impl FromRedisValue for LoRaAddr {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let u = String::from_redis_value(v)?;
        Self::from_str(&u).map_err(|e: DbErr| {
            redis::RedisError::from((redis::ErrorKind::TypeError, "LoRaAddr parse", e.to_string()))
        })
    }
}

impl std::convert::From<LoRaAddr> for sea_orm::Value {
    fn from(source: LoRaAddr) -> Self {
        source.to_string().into()
    }
}
impl sea_orm::TryGetable for LoRaAddr {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &sea_orm::QueryResult,
        idx: I,
    ) -> std::result::Result<Self, sea_orm::TryGetError> {
        <String as sea_orm::TryGetable>::try_get_by(res, idx).and_then(|v| {
            LoRaAddr::try_from(v.as_str())
                .map_err(|e| sea_orm::TryGetError::DbErr(sea_orm::DbErr::Custom(e.to_string())))
        })
    }
}
impl sea_orm::sea_query::ValueType for LoRaAddr {
    fn try_from(v: sea_orm::Value) -> std::result::Result<Self, sea_orm::sea_query::ValueTypeErr> {
        <String as sea_orm::sea_query::ValueType>::try_from(v).and_then(|v| {
            TryFrom::<&str>::try_from(v.as_str()).map_err(|_| sea_orm::sea_query::ValueTypeErr)
        })
    }
    fn type_name() -> std::string::String {
        "LoRaAddr".to_owned()
    }
    fn array_type() -> sea_orm::sea_query::ArrayType {
        String::array_type()
    }
    fn column_type() -> sea_orm::sea_query::ColumnType {
        sea_orm::prelude::ColumnType::Text
    }
}
