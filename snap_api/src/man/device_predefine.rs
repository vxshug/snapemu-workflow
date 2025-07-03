use crate::error::{ApiError, ApiResult};
use crate::load::load_config;
use crate::service::device::define::PredefineDeviceInfo;
use crate::tt;
use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct DeviceQueryClient;

#[derive(Serialize, Deserialize)]
struct Response<T> {
    pub(crate) code: i32,
    pub(crate) data: T,
}

impl DeviceQueryClient {
    pub async fn query_eui(eui: &str) -> ApiResult<Option<PredefineDeviceInfo>> {
        let config = load_config();
        let predefine = config.api.predefine.as_ref();
        if let Some(predefine) = predefine {
            let device_auth = predefine.device_auth.as_ref();
            let device_url = predefine.device_url.as_ref();
            if let (Some(device_url), Some(device_auth)) = (device_url, device_auth) {
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    device_auth.parse().map_err(|e| ApiError::Server {
                        case: "invalid auth",
                        msg: format!("{}", e).into(),
                    })?,
                );
                let client = ClientBuilder::new().default_headers(headers).build()?;
                let mut map = std::collections::HashMap::new();
                map.insert("eui", eui);
                let res = client.post(device_url).json(&map).send().await?;
                let s: Response<serde_json::Value> = res.json().await?;
                return if s.code == 200 {
                    Ok(serde_json::from_value(s.data)?)
                } else if s.code == 300 {
                    Err(crate::error::ApiError::User(tt!(
                        "messages.device.common.search_device_error",
                        eui = eui
                    )))
                } else {
                    Err(crate::error::ApiError::User(tt!(
                        "messages.device.common.search_device_error",
                        eui = eui
                    )))
                };
            }
        }
        Ok(None)
    }
}
