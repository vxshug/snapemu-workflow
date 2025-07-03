use base64::Engine;

pub trait EncodeBase64: AsRef<[u8]> {
    fn encode_base64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(self.as_ref())
    }
}

pub trait DecodeBase64: AsRef<[u8]> {
    fn decode_base64(&self) -> Result<Vec<u8>, &Self> {
        base64::engine::general_purpose::STANDARD.decode(self.as_ref()).or(Err(self))
    }
}

impl<T: AsRef<[u8]>> EncodeBase64 for T {}
impl<T: AsRef<[u8]>> DecodeBase64 for T {}
