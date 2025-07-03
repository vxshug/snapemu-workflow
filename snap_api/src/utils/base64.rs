use crate::error::{ApiError, ApiResult};
use crate::tt;
use base64::Engine;
use tracing::warn;

pub(crate) struct Base64;
const PASSWORD_TABLE: &[u8; 64] =
    b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
impl Base64 {
    pub(crate) fn standard_encode_no_pad<S: AsRef<[u8]>>(source: S) -> String {
        let source = source.as_ref();
        base64::engine::general_purpose::STANDARD_NO_PAD.encode(source)
    }
    pub(crate) fn standard_encode<S: AsRef<[u8]>>(source: S) -> String {
        let source = source.as_ref();
        base64::engine::general_purpose::STANDARD.encode(source)
    }
    pub(crate) fn standard_decode<S: AsRef<[u8]>>(src: S) -> ApiResult<Vec<u8>> {
        let src = src.as_ref();
        base64::engine::general_purpose::STANDARD.decode(src).map_err(|e| {
            warn!("{}", e);
            ApiError::User(tt!("messages.user.util.base64"))
        })
    }

    pub(crate) fn password_encode<S: AsRef<[u8]>>(source: S) -> String {
        let source = source.as_ref();
        let len = source.len();
        let mut v: Vec<u8> = Vec::with_capacity(source.len() * 4);
        let mut count = 0;
        let mut value: u32;
        while count < len {
            value = source[count] as u32;
            count += 1;
            v.push(PASSWORD_TABLE[(value & 0x3f) as usize]);
            if count < len {
                value |= (source[count] as u32) << 8;
            }
            v.push(PASSWORD_TABLE[((value >> 6) & 0x3f) as usize]);
            if count >= len {
                break;
            }
            count += 1;
            if count < len {
                value |= (source[count] as u32) << 16;
            }
            v.push(PASSWORD_TABLE[((value >> 12) & 0x3f) as usize]);
            if count >= len {
                break;
            }
            count += 1;
            v.push(PASSWORD_TABLE[((value >> 18) & 0x3f) as usize]);
        }
        String::from_utf8(v).unwrap()
    }
}
