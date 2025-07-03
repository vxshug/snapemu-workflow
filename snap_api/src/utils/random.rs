use sha2::Digest;
pub(crate) struct Rand;

impl Rand {
    pub(crate) fn string(len: u32) -> String {
        assert!(len < 64, "len < 64");
        let r: u64 = rand::random();
        let mut hasher = sha2::Sha256::new();
        hasher.update(r.to_le_bytes());
        let result = hasher.finalize();
        let s = hex::encode(result);
        s.split_at(len as usize).0.to_string()
    }
    pub(crate) fn u32() -> u32 {
        rand::random()
    }
    pub(crate) fn u64() -> u64 {
        rand::random()
    }
    pub(crate) fn u128() -> u128 {
        rand::random()
    }
}
