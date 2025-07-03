use crate::db::DbErr;
use redis::{FromRedisValue, RedisResult, RedisWrite, ToRedisArgs, Value};
use serde::de::Error;
use serde::{Deserializer, Serializer};
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::str;
use std::str::FromStr;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Key(pub lorawan::keys::AES128);

impl Deref for Key {
    type Target = lorawan::keys::AES128;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl sea_orm::sea_query::Nullable for Key {
    fn null() -> sea_orm::Value {
        sea_orm::Value::String(None)
    }
}

impl std::convert::From<Key> for sea_orm::Value {
    fn from(source: Key) -> Self {
        source.to_string().into()
    }
}
impl sea_orm::TryGetable for Key {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &sea_orm::QueryResult,
        idx: I,
    ) -> std::result::Result<Self, sea_orm::TryGetError> {
        <String as sea_orm::TryGetable>::try_get_by(res, idx).and_then(|v| {
            if v.len() != 32 {
                return Err(sea_orm::TryGetError::DbErr(sea_orm::DbErr::Custom(format!(
                    "invalid key length: {}",
                    v.len()
                ))));
            }
            let mut key = [0; 16];
            hex::decode_to_slice(&v, &mut key)
                .map_err(|e| sea_orm::TryGetError::DbErr(sea_orm::DbErr::Custom(e.to_string())))?;
            Ok(Key(lorawan::keys::AES128(key)))
        })
    }
}
impl sea_orm::sea_query::ValueType for Key {
    fn try_from(v: sea_orm::Value) -> std::result::Result<Self, sea_orm::sea_query::ValueTypeErr> {
        <String as sea_orm::sea_query::ValueType>::try_from(v).and_then(|v| {
            if v.len() != 32 {
                return Err(sea_orm::sea_query::ValueTypeErr);
            }
            let mut key = [0; 16];
            hex::decode_to_slice(&v, &mut key).map_err(|_e| sea_orm::sea_query::ValueTypeErr)?;
            Ok(Key(lorawan::keys::AES128(key)))
        })
    }
    fn type_name() -> std::string::String {
        "Key".to_owned()
    }
    fn array_type() -> sea_orm::sea_query::ArrayType {
        String::array_type()
    }
    fn column_type() -> sea_orm::sea_query::ColumnType {
        sea_orm::prelude::ColumnType::Text
    }
}

impl Key {
    pub fn new(k: [u8; 16]) -> Self {
        Self(lorawan::keys::AES128(k))
    }

    pub fn nil() -> Self {
        Self(lorawan::keys::AES128([0; 16]))
    }
}

impl serde::Serialize for Key {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}

impl<'de> serde::Deserialize<'de> for Key {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <&str as serde::Deserialize>::deserialize(deserializer)?;
        s.parse().map_err(D::Error::custom)
    }
}

impl ToRedisArgs for Key {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        self.to_string().write_redis_args(out)
    }
}

impl FromRedisValue for Key {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let u = String::from_redis_value(v)?;
        u.parse().map_err(|e: DbErr| {
            redis::RedisError::from((
                redis::ErrorKind::ResponseError,
                "redis parse key",
                e.to_string(),
            ))
        })
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode_upper(self.0 .0))
    }
}

impl TryFrom<&str> for Key {
    type Error = DbErr;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 32 {
            return Err(DbErr::Len(format!("Key most 32 byte, found '{}'", value)));
        }
        let mut b = [0; 16];
        hex::decode_to_slice(value, &mut b).map_err(|_| DbErr::Parse)?;

        Ok(Self::new(b))
    }
}

impl FromStr for Key {
    type Err = DbErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Key::try_from(s)
    }
}
