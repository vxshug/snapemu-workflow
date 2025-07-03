pub(crate) mod data;
mod decode;
mod downlink;
pub(crate) mod lora;
mod mq;
pub mod mqtt;
pub mod redis_client;
pub mod influxdb;
pub(crate) mod gw;
pub mod device_active;

pub use downlink::DownlinkManager;

use common_define::product::ProductType;
pub use decode::DecodeManager;
pub use mq::MQ;

pub(crate) type Id = common_define::Id;
#[derive(
    derive_more::From,
    Debug,
    Clone,
    Copy,
    serde::Serialize,
    serde::Deserialize,
    redis_macros::FromRedisValue,
    redis_macros::ToRedisArgs
)]
#[serde(transparent)]
pub(crate) struct ProductTypeLocal(ProductType);
