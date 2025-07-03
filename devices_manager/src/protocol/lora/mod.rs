use lorawan::keys::AES128;
use std::str::FromStr;

use crate::man::data::DataError;

pub(crate) mod data;
pub(crate) mod join_accept;
pub(crate) mod join_request;
pub(crate) mod parse;
pub(crate) mod payload;
pub mod source;

#[derive(Debug)]
pub(crate) struct AppKey(AES128);

impl AppKey {
    pub(crate) fn key(&self) -> &AES128 {
        &self.0
    }
}

impl FromStr for AppKey {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key = match s.len() {
            32 => {
                let mut b = [0; 16];
                hex::decode_to_slice(s, &mut b)?;
                b
            }
            _ => {
                return Err(DataError::from("appkey most 16 bytes"));
            }
        };
        Ok(Self(AES128::from(key)))
    }
}
