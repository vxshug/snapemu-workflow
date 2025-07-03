use common_define::decode::DecodeLang;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DecodeResponse {
    pub(crate) result: String,
    pub(crate) state: bool,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DecodeRequest {
    pub(crate) script: String,
    pub(crate) bytes: String,
    pub(crate) lang: DecodeLang,
}
