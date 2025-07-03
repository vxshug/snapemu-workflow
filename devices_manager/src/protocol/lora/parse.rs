use std::fmt::Debug;

use crate::man::data::DataError;
use crate::protocol::lora::join_request::RequestJoin;
use crate::protocol::lora::payload::LoRaPayload;
use base64::Engine;
use lorawan::parser::{parse, PhyPayload};

pub(crate) type RBytes = Vec<u8>;

#[derive(Debug)]
pub(crate) enum LoraPhy {
    Request(RequestJoin),
    Payload(LoRaPayload),
}

pub(crate) struct LoraMacDecode;

impl LoraMacDecode {
    pub(crate) fn switch<B: AsRef<[u8]>>(base: B) -> Result<LoraPhy, DataError> {
        let data = base64::engine::general_purpose::STANDARD.decode(base)?;
        Self::inner_parse(data)
    }

    fn inner_parse(data: Vec<u8>) -> Result<LoraPhy, DataError> {
        let result = parse(data)?;
        match result {
            PhyPayload::JoinRequest(re) => Ok(LoraPhy::Request(RequestJoin::new(re))),
            PhyPayload::Data(data) => {
                let payload = LoRaPayload::new(data)?;
                Ok(LoraPhy::Payload(payload))
            }
            _ => Err(DataError::from("PhyPayload Parse error")),
        }
    }
}
//
// #[derive(Clone)]
// pub(crate) struct Key(lorawan::keys::AES128);
//
// impl Debug for Key {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", hex::encode(self.0.0))
//     }
// }
//
// impl Deref for Key {
//     type Target = lorawan::keys::AES128;
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }
//
//
// impl Key {
//     pub(crate) fn new(key: [u8; 16]) -> Self {
//         Self(lorawan::keys::AES128(key))
//     }
//
//     pub(crate) fn string(&self) -> String {
//         hex::encode_upper(&self.0.0)
//     }
// }
//
// impl FromStr for Key {
//     type Err = DataError;
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         let mut key = [0; 16];
//         hex::decode_to_slice(s, &mut key)
//         .map_err(|s| s.to_string())?;
//         Ok(Self(lorawan::keys::AES128(key)))
//     }
// }
//
// impl redis::FromRedisValue for Key {
//     fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
//         let mut key = [0; 16];
//         match *v {
//             redis::Value::Data(ref bytes) => {
//                 hex::decode_to_slice(bytes, &mut key)
//                 .map_err(|s| redis::RedisError::from((redis::ErrorKind::TypeError, "str not match", s.to_string())))?;
//                 Ok(Self(lorawan::keys::AES128(key)))
//             }
//             _ => Err((redis::ErrorKind::TypeError, "Response type not string compatible.").into())
//         }
//     }
// }
//
// impl serde::Serialize for Key {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
//        self.string().serialize(serializer)
//     }
// }
//
// impl<'de> serde::Deserialize<'de> for Key {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
//         let s  = String::deserialize(deserializer)?;
//         s.parse().map_err(|e: DataError| serde::de::Error::custom(e.msg))
//     }
// }
//
// impl redis::ToRedisArgs for Key {
//     fn write_redis_args<W>(&self, out: &mut W)
//         where
//             W: ?Sized + redis::RedisWrite {
//         out.write_arg(hex::encode_upper(&self.0.0).as_bytes())
//     }
// }
//
// #[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, redis_macros::FromRedisValue, redis_macros::ToRedisArgs)]
// #[serde(transparent)]
// pub(crate) struct LoRaDevAddr(u32);
//
// impl LoRaDevAddr {
//     pub(crate) const NWKADDR_N: u8 = 24;
//     pub(crate) fn is_abp_addr(&self) -> bool {
//         (self.0 >> Self::NWKADDR_N) > 0
//     }
//     pub(crate) fn is_otaa_addr(&self) -> bool {
//         !self.is_abp_addr()
//     }
//     pub(crate) fn addr(&self) -> u32 {
//         self.0
//     }
//
//     pub(crate) fn to_bytes(&self) -> [u8; 4] {
//         self.0.to_le_bytes()
//     }
// }
//
// impl Display for LoRaDevAddr {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//        write!(f, "{}", hex::encode_upper(self.0.to_be_bytes()))
//     }
// }
//
// impl From<u32> for LoRaDevAddr {
//     fn from(value: u32) -> Self {
//         Self(value)
//     }
// }
//
// impl FromStr for LoRaDevAddr {
//     type Err = DataError;
//    fn from_str(s: &str) -> Result<Self, Self::Err> {
//         let mut buf = [0; 4];
//         hex::decode_to_slice(s, &mut buf)
//             .map_err(|s| s.to_string())?;
//         Ok(Self(u32::from_be_bytes(buf)))
//    }
// }
