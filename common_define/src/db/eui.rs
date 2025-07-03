use crate::db::DbErr;
use redis::{FromRedisValue, RedisResult, RedisWrite, ToRedisArgs, Value};
use serde::de::Error;
use serde::{Deserializer, Serializer};
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

#[derive(derive_more::From, Clone, Copy, Hash, Eq, PartialEq, Default, Ord, PartialOrd)]
pub struct Eui(pub u64);

impl Debug for Eui {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl std::convert::From<Eui> for sea_orm::Value {
    fn from(source: Eui) -> Self {
        source.to_string().into()
    }
}
impl sea_orm::TryGetable for Eui {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &sea_orm::QueryResult,
        idx: I,
    ) -> std::result::Result<Self, sea_orm::TryGetError> {
        <String as sea_orm::TryGetable>::try_get_by(res, idx).and_then(|v| {
            Eui::try_from(v.as_str())
                .map_err(|e| sea_orm::TryGetError::DbErr(sea_orm::DbErr::Custom(e.to_string())))
        })
    }
}
impl sea_orm::sea_query::ValueType for Eui {
    fn try_from(v: sea_orm::Value) -> std::result::Result<Self, sea_orm::sea_query::ValueTypeErr> {
        <String as sea_orm::sea_query::ValueType>::try_from(v).and_then(|v| {
            TryFrom::<&str>::try_from(v.as_str()).map_err(|_| sea_orm::sea_query::ValueTypeErr)
        })
    }
    fn type_name() -> std::string::String {
        "Eui".to_owned()
    }
    fn array_type() -> sea_orm::sea_query::ArrayType {
        String::array_type()
    }
    fn column_type() -> sea_orm::sea_query::ColumnType {
        sea_orm::prelude::ColumnType::Text
    }
}

impl sea_orm::TryFromU64 for Eui {
    fn try_from_u64(n: u64) -> Result<Self, sea_orm::DbErr> {
        Ok(Self(n))
    }
}

impl Eui {
    pub fn new(eui: u64) -> Self {
        Self(eui)
    }

    pub fn random() -> Self {
        Self(rand::random())
    }

    pub fn to_bytes(&self) -> [u8; 8] {
        self.0.to_le_bytes()
    }
    pub fn to_be_bytes(&self) -> [u8; 8] {
        self.0.to_be_bytes()
    }
    pub fn from_be_bytes(s: &[u8]) -> Option<Self> {
        if s.len() < 8 {
            return None;
        }
        let eui = &s[0..8];
        let mut t = [0; 8];
        t.copy_from_slice(eui);
        Some(Self(u64::from_be_bytes(t)))
    }
}

impl From<Eui> for u64 {
    fn from(value: Eui) -> Self {
        value.0
    }
}

impl From<[u8; 8]> for Eui {
    fn from(value: [u8; 8]) -> Self {
        Self(u64::from_le_bytes(value))
    }
}

impl serde::Serialize for Eui {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}

impl<'de> serde::Deserialize<'de> for Eui {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <&str as serde::Deserialize>::deserialize(deserializer)?;
        s.parse().map_err(|e: DbErr| Error::custom(e.to_string()))
    }
}

impl Display for Eui {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode_upper(self.0.to_be_bytes()))
    }
}

impl ToRedisArgs for Eui {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        self.to_string().write_redis_args(out)
    }
}

impl FromRedisValue for Eui {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let u = String::from_redis_value(v)?;
        Self::from_str(&u).map_err(|e: DbErr| {
            redis::RedisError::from((redis::ErrorKind::TypeError, "Eui parse", e.to_string()))
        })
    }
}

impl FromStr for Eui {
    type Err = DbErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Eui::try_from(s)
    }
}

impl TryFrom<&str> for Eui {
    type Error = DbErr;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 16 {
            return Err(DbErr::Len(format!("Eui most 16 byte, found '{}'", value)));
        }
        let mut b = [0; 8];
        hex::decode_to_slice(value, &mut b).map_err(|_| DbErr::Parse)?;

        Ok(Self(u64::from_be_bytes(b)))
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Eui;
    use std::str::FromStr;

    #[test]
    fn test_convert_str() {
        let s = "1231231231231231";
        let eui = Eui::from_str(s).unwrap();
        assert_eq!(s, eui.to_string());
    }
}
