use crate::sea_string_type;
use serde::{Deserializer, Serializer};
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Copy,
    Clone,
    Debug,
    strum::AsRefStr,
    strum::EnumString
)]
pub enum ValueType {
    Array,
    F64,
    F32,
    Bool,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
}

#[derive(
    PartialEq,
    serde::Deserialize,
    serde::Serialize,
    redis_macros::FromRedisValue,
    redis_macros::ToRedisArgs,
    Clone,
    Copy,
    Debug,
    strum::AsRefStr,
    strum::EnumString,
    Eq
)]
pub enum LoRaRegion {
    EU868,
    US915,
    CN779,
    EU433,
    AU915,
    CN470,
    AS923_1,
    AS923_2,
    AS923_3,
    KR920,
    IN865,
    RU864,
}

sea_string_type!(LoRaRegion);

#[derive(
    serde::Serialize,
    serde::Deserialize,
    redis_macros::FromRedisValue,
    redis_macros::ToRedisArgs,
    Clone,
    Copy,
    Debug,
    strum::AsRefStr,
    strum::EnumString,
    Eq,
    PartialEq
)]
pub enum LoRaJoinType {
    OTAA,
    ABP,
}

sea_string_type!(LoRaJoinType);

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
    Copy,
    redis_macros::FromRedisValue,
    redis_macros::ToRedisArgs
)]
#[serde(transparent)]
pub struct LoRaDevAddr(u32);

impl LoRaDevAddr {
    pub(crate) const NWKADDR_N: u8 = 24;
    pub fn is_abp_addr(&self) -> bool {
        (self.0 >> Self::NWKADDR_N) > 0
    }
    pub fn is_otaa_addr(&self) -> bool {
        !self.is_abp_addr()
    }
    pub fn addr(&self) -> u32 {
        self.0
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

impl Display for LoRaDevAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode_upper(self.0.to_be_bytes()))
    }
}

impl From<u32> for LoRaDevAddr {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl FromStr for LoRaDevAddr {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut buf = [0; 4];
        hex::decode_to_slice(s, &mut buf).map_err(|s| s.to_string())?;
        Ok(Self(u32::from_be_bytes(buf)))
    }
}

#[derive(Clone)]
pub struct Key(lorawan::keys::AES128);

impl Debug for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0 .0))
    }
}

impl Deref for Key {
    type Target = lorawan::keys::AES128;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Key {
    pub(crate) fn string(&self) -> String {
        hex::encode_upper(self.0 .0)
    }
}

impl FromStr for Key {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut key = [0; 16];
        hex::decode_to_slice(s, &mut key).map_err(|s| s.to_string())?;
        Ok(Self(lorawan::keys::AES128(key)))
    }
}

impl redis::FromRedisValue for Key {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
        let mut key = [0; 16];
        match *v {
            redis::Value::BulkString(ref bytes) => {
                hex::decode_to_slice(bytes, &mut key).map_err(|s| {
                    redis::RedisError::from((
                        redis::ErrorKind::TypeError,
                        "str not match",
                        s.to_string(),
                    ))
                })?;
                Ok(Self(lorawan::keys::AES128(key)))
            }
            _ => Err((redis::ErrorKind::TypeError, "Response type not string compatible.").into()),
        }
    }
}

impl serde::Serialize for Key {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.string().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Key {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl redis::ToRedisArgs for Key {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        out.write_arg(hex::encode_upper(self.0 .0).as_bytes())
    }
}
