use influxdb2_structmap::GenericMap;
use common_define::decode::Value;

pub(crate) struct DataService;

pub(crate) mod query;
pub(crate) mod update;

#[derive(Debug, Default)]
pub struct DeviceData {
    id: i64,
    name: String,
    unit: String,
    value: Value,
    time: i64,
}

impl influxdb2::FromMap for DeviceData {
    fn from_genericmap(map: GenericMap) -> Self {
        let mut map = map;
        let mut this = Self::default();
        if let Some(influxdb2_structmap::value::Value::String(id)) = map.remove("id") {
            this.id = id.parse::<i64>().unwrap_or_default();
        }
        if let Some(influxdb2_structmap::value::Value::String(name)) = map.remove("name") {
            this.name = name;
        }
        if let Some(influxdb2_structmap::value::Value::String(unit)) = map.remove("unit") {
            this.unit = unit;
        }
        if let Some(value) = map.remove("value") {
            match value {
                influxdb2_structmap::value::Value::Long(i) => { this.value = Value::Int(i); }
                influxdb2_structmap::value::Value::Double(f) => { this.value = Value::Float(f.0); }
                influxdb2_structmap::value::Value::Bool(b) => { this.value = Value::Bool(b); }
                _ => {}
            }
        }
        if let Some(influxdb2_structmap::value::Value::TimeRFC(time)) = map.remove("_time") {
            this.time = time.timestamp_millis();
        }
        this
    }
}