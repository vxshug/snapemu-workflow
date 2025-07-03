use crate::Id;

pub fn last_device_data_key(id: Id) -> String {
    format!("data:{}:last", id)
}
