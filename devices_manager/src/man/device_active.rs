use std::collections::BTreeMap;
use std::sync::Mutex;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use common_define::Id;
use crate::{DeviceResult, GLOBAL_STATE};

static DEVICE_ACTIVE: Mutex<Option<BTreeMap<Id, chrono::DateTime<chrono::Utc>>>> = Mutex::new(None);

pub struct DeviceActive;

impl DeviceActive {

    pub fn insert(device_id: Id, timestamp: chrono::DateTime<chrono::Utc>) {
        let mut map = DEVICE_ACTIVE.lock().unwrap();
        if let Some(map) = &mut *map {
            map.insert(device_id, timestamp);
        } else {
            let mut nmap = BTreeMap::new();
            nmap.insert(device_id, timestamp);
            map.replace(nmap);
        }
    }

    pub async fn remove() -> DeviceResult {
        let mut map = DEVICE_ACTIVE.lock().unwrap();
        if let Some(map) = map.take() {
            let conn = &GLOBAL_STATE.db;
            if !map.is_empty() {
                let items: Vec<_> = map.iter().map(|(k, v)| format!("({},{})", k.0, v)).collect();
                let sql = format!("UPDATE snap_devices SET active_time = time.time FROM ( VALUES {}) AS time(id, time) WHERE snap_devices.id = time.id;", items.join(","));
                conn.execute(Statement::from_string(DatabaseBackend::Postgres, sql)).await?;
            }
        }
        Ok(())
    }
}