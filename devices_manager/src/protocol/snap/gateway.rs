use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct UpJson {
    pub token: u16,
    pub data: String,
    pub rssi: i32,
    pub freq: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DownJson {
    pub token: u16,
    pub freq: f32,
    pub data: String,
}
