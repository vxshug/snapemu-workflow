use serde::Serialize;

#[derive(Serialize)]
#[serde(transparent)]
pub struct PlatformLog(serde_json::Value);

impl PlatformLog {
    pub const TOPIC: &'static str = "PLATFORM_LOGS";
    pub fn new<T: serde::Serialize>(v: T) -> Option<Self> {
        serde_json::to_value(v).map(PlatformLog).ok()
    }
}
