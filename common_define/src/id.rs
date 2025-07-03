use crate::db::DbErr;
use redis::{FromRedisValue, RedisResult, RedisWrite, ToRedisArgs, Value};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(
    derive_more::From,
    Debug,
    Clone,
    Copy,
    serde::Serialize,
    serde::Deserialize,
    Hash,
    Eq,
    PartialEq,
    Default,
    Ord,
    PartialOrd,
    utoipa::ToSchema
)]
#[serde(transparent)]
pub struct Id(pub u64);

impl sea_orm::sea_query::value::with_array::NotU8 for Id {}
impl sea_orm::sea_query::Nullable for Id {
    fn null() -> sea_orm::Value {
        sea_orm::Value::BigInt(None)
    }
}

impl From<Id> for sea_orm::Value {
    fn from(source: Id) -> Self {
        source.0.into()
    }
}

impl From<&Id> for sea_orm::Value {
    fn from(source: &Id) -> Self {
        source.0.into()
    }
}

impl sea_orm::TryGetable for Id {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &sea_orm::QueryResult,
        idx: I,
    ) -> std::result::Result<Self, sea_orm::TryGetError> {
        <i64 as sea_orm::TryGetable>::try_get_by(res, idx).map(|v| Id(v as u64))
    }
}
impl sea_orm::sea_query::ValueType for Id {
    fn try_from(v: sea_orm::Value) -> std::result::Result<Self, sea_orm::sea_query::ValueTypeErr> {
        <i64 as sea_orm::sea_query::ValueType>::try_from(v).map(|v| Id(v as u64))
    }
    fn type_name() -> std::string::String {
        "Id".to_owned()
    }
    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::BigInt
    }
    fn column_type() -> sea_orm::sea_query::ColumnType {
        sea_orm::prelude::ColumnType::BigInteger
    }
}

impl sea_orm::TryFromU64 for Id {
    fn try_from_u64(n: u64) -> Result<Self, sea_orm::DbErr> {
        Ok(Self(n))
    }
}

impl Id {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn random() -> Self {
        Self(rand::random())
    }
}

impl From<Id> for u64 {
    fn from(value: Id) -> Self {
        value.0
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode_upper(self.0.to_be_bytes()))
    }
}

impl ToRedisArgs for Id {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        self.0.write_redis_args(out)
    }
}

impl FromRedisValue for Id {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let u = u64::from_redis_value(v)?;
        Ok(Self(u))
    }
}

impl FromStr for Id {
    type Err = DbErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Id::try_from(s)
    }
}

impl TryFrom<&str> for Id {
    type Error = DbErr;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 16 {
            return Err(DbErr::Len(format!("id is error: {}", value)));
        }
        let mut b = [0; 8];
        hex::decode_to_slice(value, &mut b).map_err(|_| DbErr::Parse)?;

        Ok(Self(u64::from_be_bytes(b)))
    }
}
