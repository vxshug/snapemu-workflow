use sha2::{Digest, Sha256, Sha512};

use super::base64::Base64;

pub(crate) struct Hash;

impl Hash {
    pub(crate) fn sha256<D: AsRef<[u8]>>(data: D) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let res = hasher.finalize();
        res.to_vec()
    }
    pub(crate) fn sha256_base64<D: AsRef<[u8]>>(data: D) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let res = hasher.finalize();
        Base64::standard_encode(res)
    }
    pub(crate) fn sha512_base64<D: AsRef<[u8]>>(data: D) -> String {
        let mut hasher = Sha512::new();
        hasher.update(data);
        let res = hasher.finalize();
        Base64::standard_encode(res)
    }
}
