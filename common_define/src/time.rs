use redis::{RedisResult, RedisWrite, Value};
use std::fmt::{Debug, Display, Formatter};
use chrono::Timelike;

#[derive(
    sea_orm::DeriveValueType,
    derive_more::From,
    Clone,
    Copy,
    serde::Deserialize,
    serde::Serialize,
    PartialEq,
    PartialOrd,
    Eq
)]
#[serde(try_from = "u64", into = "u64")]
pub struct Timestamp(chrono::DateTime<chrono::Utc>);

impl sea_orm::sea_query::Nullable for Timestamp {
    fn null() -> sea_orm::Value {
        sea_orm::Value::ChronoDateTimeUtc(None)
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.timestamp_millis(), f)
    }
}
impl Debug for Timestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Timestamp {
    pub fn now() -> Self {
        Self(chrono::Utc::now())
    }

    pub fn timestamp_millis(&self) -> u64 {
        self.0.timestamp_millis() as u64
    }
    pub fn timestamp_micros(&self) -> u64 {
        self.0.timestamp_micros() as u64
    }

    pub fn from_timestamp_millis(value: u64) -> Option<Self> {
        chrono::DateTime::from_timestamp_millis(value as i64).map(Self)
    }

    pub fn to_rfc3339(&self) -> String {
        self.0.to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
    }
}

impl std::ops::Sub<Self> for Timestamp {
    type Output = chrono::Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl std::ops::Sub<chrono::Duration> for Timestamp {
    type Output = Self;

    fn sub(self, rhs: chrono::Duration) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl std::ops::Add<chrono::Duration> for Timestamp {
    type Output = Self;

    fn add(self, rhs: chrono::Duration) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl From<Timestamp> for u64 {
    fn from(value: Timestamp) -> Self {
        value.timestamp_millis()
    }
}

impl TryFrom<u64> for Timestamp {
    type Error = String;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::from_timestamp_millis(value).ok_or(format!("Invalid timestamp: {}", value))
    }
}

impl redis::ToRedisArgs for Timestamp {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        self.timestamp_millis().write_redis_args(out)
    }
}
impl redis::FromRedisValue for Timestamp {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let u = u64::from_redis_value(v)?;
        Self::from_timestamp_millis(u)
            .ok_or(redis::RedisError::from((redis::ErrorKind::TypeError, "time from error")))
    }
}
