use crate::man::data::{DataError, ValueType};
use common_define::db::DbDecodeData;
use derive_new::new;
use rquickjs::{CatchResultExt, Function};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use snap_model::ModelSource;

fn check_data_length(bytes: &[u8]) -> Result<(), DataError> {
    let mut check_len = 0;
    let mut prefix = 0;
    let len = bytes.len();
    loop {
        let sensor_len =
            *bytes.get(check_len + 2).ok_or(format!("data length error: {:?}", bytes))?;
        check_len += sensor_len as usize + 3;
        if check_len == len {
            break;
        }
    }
    check_len = 0;
    prefix = 0;
    loop {
        check_len += 2;
        prefix += 2;
        let sensor_len = *bytes.get(check_len).ok_or(format!("data length error: {:?}", bytes))?;
        check_len += sensor_len as usize + 1;
        prefix += 1;

        if check_len > len || check_len <= prefix {
            return Err(DataError::from("length error"));
        } else {
            let mut s = 0;
            loop {
                let mut type_len = 0;
                let pk_type = *bytes.get(prefix).ok_or(DataError::from("length error"))? & 0x0F;
                let pk_type: ValueType = pk_type.try_into()?;
                match pk_type {
                    ValueType::Array => {
                        type_len =
                            *bytes.get(prefix + 1).ok_or(DataError::from("length error"))? + 1;
                    }
                    ValueType::F64 => {
                        type_len = 8;
                    }
                    ValueType::F32 => {
                        type_len = 4;
                    }
                    ValueType::Bool => {
                        type_len = 1;
                    }
                    ValueType::I8 => {
                        type_len = 1;
                    }
                    ValueType::U8 => {
                        type_len = 1;
                    }
                    ValueType::I16 => {
                        type_len = 2;
                    }
                    ValueType::U16 => {
                        type_len = 2;
                    }
                    ValueType::I32 => {
                        type_len = 4;
                    }
                    ValueType::U32 => {
                        type_len = 4;
                    }
                };
                s += type_len + 1;
                prefix += type_len as usize + 1;
                if sensor_len == s {
                    break;
                }
            }
        }
        if len == check_len {
            break;
        }
    }
    Ok(())
}

#[derive(new, Debug)]
pub(crate) struct DeviceBattery {
    pub(crate) battery: u8,
    pub(crate) charge: bool,
}

#[derive(Debug)]
pub(crate) struct DeviceIO {
    pub(crate) pin: i16,
    pub(crate) modify: bool,
    pub(crate) mode: bool,
    pub(crate) value: bool,
}

#[derive(Default, influxdb2_derive::WriteDataPoint)]
#[measurement = "device_data"]
struct TsdbData {
    #[influxdb(tag)]
    device: u64,
    #[influxdb(field)]
    value: i64,
    #[influxdb(timestamp)]
    time: i64,
}

#[derive(Default, Debug)]
pub struct DecodeDataDecoded {
    pub data: Vec<common_define::decode::DecodeData>,
    pub io: Vec<DeviceIO>,
    pub status: Option<DeviceBattery>,
}

pub(crate) fn up_data_decode(bytes: &[u8]) -> Result<DecodeDataDecoded, DataError> {
    let len = bytes.len();
    let mut current_len = 0;

    let mut r = DecodeDataDecoded::default();
    loop {
        if current_len == len {
            break;
        }
        current_len += 2; // sensor id
        if current_len >= len {
            return Err(DataError::from("sensor id not found".to_string()));
        }
        let sensor_id = (bytes[current_len - 2] as u32) | ((bytes[current_len - 1] as u32) << 8);
        current_len += 1;
        if current_len >= len {
            return Err(DataError::from("data length not found".to_string()));
        }
        let data_len = bytes[current_len - 1] as usize;
        current_len += data_len;
        if current_len > len {
            return Err(DataError::from("data length so mush".to_string()));
        }
        let mut parse_data_len = 0;
        let bytes = &bytes[(current_len - data_len)..current_len];
        if sensor_id == 0 {
            if data_len == 2 {
                r.status.replace(DeviceBattery::new(bytes[1], false));
            } else if data_len == 4 {
                r.status.replace(DeviceBattery::new(bytes[1], bytes[3] == 1));
            }
            continue;
        }
        if sensor_id == 7 {
            if data_len == 11 {
                let io_num = bytes[1];
                if io_num <= 16 {
                    let modify = bytes[3] as u16 | ((bytes[4] as u16) << 8);
                    let mode = bytes[6] as u16 | ((bytes[7] as u16) << 8);
                    let status = bytes[9] as u16 | ((bytes[10] as u16) << 8);
                    for i in 0..io_num {
                        let s = DeviceIO {
                            pin: i as i16,
                            modify: modify & (1 << i) != 0,
                            mode: mode & (1 << i) != 0,
                            value: status & (1 << i) != 0,
                        };
                        r.io.push(s);
                    }
                }
            }
            continue;
        }
        loop {
            if data_len == 0 {
                break;
            };
            if parse_data_len >= data_len {
                break;
            }
            let pk_id = (bytes[parse_data_len] >> 4) as u32;
            let pk_type: ValueType = (bytes[parse_data_len] & 0x0f).try_into()?;
            parse_data_len += 1;
            let data = match pk_type {
                ValueType::Array => {
                    if parse_data_len > data_len {
                        return Err(DataError::from(format!(
                            "not found array length: {:?}",
                            bytes
                        )));
                    }
                    let arr_len = bytes[parse_data_len] as usize;
                    parse_data_len += 1;
                    if parse_data_len + arr_len > data_len {
                        return Err(DataError::from(format!("data length error: {:?}", bytes)));
                    }
                    let mut v = Vec::with_capacity(arr_len);
                    for item in &bytes[parse_data_len..parse_data_len + arr_len] {
                        v.push(*item);
                    }
                    parse_data_len += arr_len;
                    0.into()
                }
                ValueType::F64 => {
                    let mut u = [0; 8];
                    if parse_data_len + 8 > data_len {
                        return Err(DataError::from(format!("not found f64 length: {:?}", bytes)));
                    }
                    u.copy_from_slice(&bytes[parse_data_len..parse_data_len + 8]);
                    parse_data_len += 8;
                    f64::from_le_bytes(u).into()
                }
                ValueType::F32 => {
                    let mut u = [0; 4];
                    if parse_data_len + 4 > data_len {
                        return Err(DataError::from(format!("not found f32 length: {:?}", bytes)));
                    }
                    u.copy_from_slice(&bytes[parse_data_len..parse_data_len + 4]);
                    parse_data_len += 4;
                    f32::from_le_bytes(u).into()
                }
                ValueType::Bool => {
                    if parse_data_len + 1 > data_len {
                        return Err(DataError::from(format!("not found bool length: {:?}", bytes)));
                    }
                    let u = bytes[parse_data_len];
                    parse_data_len += 1;
                    (u != 0).into()
                }
                ValueType::I8 => {
                    if parse_data_len + 1 > data_len {
                        return Err(DataError::from(format!("not found i8 length: {:?}", bytes)));
                    }
                    let u = bytes[parse_data_len];
                    parse_data_len += 1;
                    (u as i8).into()
                }
                ValueType::U8 => {
                    if parse_data_len + 1 > data_len {
                        return Err(DataError::from(format!("not found u8 length: {:?}", bytes)));
                    }
                    let u = bytes[parse_data_len];
                    parse_data_len += 1;
                    u.into()
                }
                ValueType::I16 => {
                    let mut u = [0; 2];
                    if parse_data_len + 2 > data_len {
                        return Err(DataError::from(format!("not found i16 length: {:?}", bytes)));
                    }
                    u.copy_from_slice(&bytes[parse_data_len..parse_data_len + 2]);
                    parse_data_len += 2;
                    i16::from_le_bytes(u).into()
                }
                ValueType::U16 => {
                    let mut u = [0; 2];
                    if parse_data_len + 2 > data_len {
                        return Err(DataError::from(format!("not found u16 length: {:?}", bytes)));
                    }
                    u.copy_from_slice(&bytes[parse_data_len..parse_data_len + 2]);
                    parse_data_len += 2;
                    u16::from_le_bytes(u).into()
                }
                ValueType::I32 => {
                    let mut u = [0; 4];
                    if parse_data_len + 4 > data_len {
                        return Err(DataError::from(format!("not found i32 length: {:?}", bytes)));
                    }
                    u.copy_from_slice(&bytes[parse_data_len..parse_data_len + 4]);
                    parse_data_len += 4;
                    i32::from_le_bytes(u).into()
                }
                ValueType::U32 => {
                    let mut u = [0; 4];
                    if parse_data_len + 4 > data_len {
                        return Err(DataError::from(format!("not found u32 length: {:?}", bytes)));
                    }
                    u.copy_from_slice(&bytes[parse_data_len..parse_data_len + 4]);
                    parse_data_len += 4;
                    u32::from_le_bytes(u).into()
                }
            };
            let data_id = sensor_id << 4 | pk_id;
            r.data.push(common_define::decode::DecodeData::new(data_id, data));
        }
    }
    Ok(r)
}

#[derive(Debug)]
pub struct RawData {
    bytes: Vec<u8>,
}

impl RawData {
    pub fn new<B: Into<Vec<u8>>>(bytes: B) -> Self {
        Self { bytes: bytes.into() }
    }
}

const JS_FUNCTION_NAME: &str = "decodeUplink";

#[derive(Debug, serde::Serialize)]
pub struct DecodeData {
    pub data: Vec<DecodeDataItem>,
}

impl DecodeData {
    pub fn new<S: ModelSource>(decode_data: Vec<common_define::decode::DecodeData>, map: S) -> Self {
        let mut data = Vec::new();
        for item in decode_data {
            let data_map = map.get_data_name(item.i);
            data.push(DecodeDataItem {
                v: item.v,
                i: item.i as _,
                name: data_map.name.format_tag(),
                unit: data_map.unit.to_string()
            });
        }
        Self {
            data
        }
    }
}

impl From<DecodeData> for DbDecodeData {
    fn from(value: DecodeData) -> Self {
        Self(
            value
                .data
                .into_iter()
                .map(|it| common_define::decode::DecodeData::new(it.i as u32, it.v))
                .collect(),
        )
    }
}

fn js_to_serde_value(data: &rquickjs::Value) -> Option<common_define::decode::Value> {
    if let Some(b) = data.as_bool() {
        return Some(common_define::decode::Value::from(b));
    };
    if let Some(b) = data.as_int() {
        return Some(common_define::decode::Value::from(b));
    };
    if let Some(b) = data.as_float() {
        return Some(common_define::decode::Value::from(b));
    };
    None
}

#[derive(Debug, serde::Serialize)]
pub struct DecodeDataItem {
    pub v: common_define::decode::Value,
    pub i: i32,
    pub name: String,
    pub unit: String,
}
impl<'js> rquickjs::FromJs<'js> for DecodeDataItem {
    fn from_js(_ctx: &rquickjs::Ctx<'js>, value: rquickjs::Value<'js>) -> rquickjs::Result<Self> {
        let obj = rquickjs::Object::from_value(value)?;
        let data: rquickjs::Value = obj.get("data")?;
        let data = js_to_serde_value(&data).ok_or(rquickjs::Error::FromJs {
            from: "data",
            to: "number or bool",
            message: Some(format!("data: {:?}, not a number or bool", data)),
        })?;
        let id: i32 = obj.get("id").map_err(|_| rquickjs::Error::FromJs {
            from: "",
            to: "",
            message: Some("id most number".to_string()),
        })?;
        let name: String = obj.get("name").map_err(|_| rquickjs::Error::FromJs {
            from: "",
            to: "",
            message: Some("name most number".to_string()),
        })?;
        let unit: String = obj.get("unit").map_err(|_| rquickjs::Error::FromJs {
            from: "",
            to: "",
            message: Some("unit most number".to_string()),
        })?;
        Ok(Self { v: data, i: id, name, unit })
    }
}
impl<'js> rquickjs::FromJs<'js> for DecodeData {
    fn from_js(_ctx: &rquickjs::Ctx<'js>, value: rquickjs::Value<'js>) -> rquickjs::Result<Self> {
        let obj = rquickjs::Object::from_value(value).map_err(|_| rquickjs::Error::FromJs {
            from: "",
            to: "",
            message: Some(format!(
                "{} function most return a Object contains an array type named data",
                JS_FUNCTION_NAME
            )),
        })?;
        let data: Vec<DecodeDataItem> = obj.get("data")?;
        Ok(Self { data })
    }
}

impl<'js> rquickjs::IntoJs<'js> for RawData {
    fn into_js(self, ctx: &rquickjs::Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> {
        rquickjs::Object::new(ctx.clone())
            .and_then(|obj| obj.prop("bytes", self.bytes).map(|_| obj))
            .into_js(ctx)
    }
}

struct JsContext {
    hand: tokio::task::JoinHandle<()>,
    ctx: rquickjs::Context,
}

impl Drop for JsContext {
    fn drop(&mut self) {
        self.hand.abort();
    }
}

struct JsManagerInner {
    runtime: rquickjs::Runtime,
    notify: Arc<AtomicBool>,
    timeout: u64,
}

pub struct JsManager {
    inner: Arc<JsManagerInner>,
}

impl Clone for JsManager {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

#[derive(Debug)]
pub enum JsDecodeError {
    Unknown(String),
    TimeOut { stack: Option<String> },
    Return(String),
    Export(String),
}

impl From<rquickjs::Error> for JsDecodeError {
    fn from(value: rquickjs::Error) -> Self {
        match value {
            rquickjs::Error::FromJs { message, .. } => Self::Return(message.unwrap_or_default()),
            e => Self::Unknown(e.to_string()),
        }
    }
}
impl<'js> From<rquickjs::CaughtError<'js>> for JsDecodeError {
    fn from(value: rquickjs::CaughtError<'js>) -> Self {
        match value {
            rquickjs::CaughtError::Error(e) => Self::from(e),
            rquickjs::CaughtError::Exception(ex) => match ex.message() {
                None => Self::Unknown(ex.to_string()),
                Some(s) => {
                    if s == "interrupted" {
                        Self::TimeOut { stack: ex.stack() }
                    } else {
                        Self::Unknown(s)
                    }
                }
            },
            rquickjs::CaughtError::Value(v) => Self::Unknown(format!("value {:?}", v)),
        }
    }
}
impl JsManager {
    pub fn new() -> Self {
        let runtime = rquickjs::Runtime::new().unwrap();
        runtime.set_memory_limit(2_000_000);
        let timeout = Arc::new(AtomicBool::new(false));
        let notify = timeout.clone();
        runtime
            .set_interrupt_handler(Some(Box::new(move || timeout.swap(false, Ordering::Relaxed))));
        Self { inner: Arc::new(JsManagerInner { runtime, notify, timeout: 1000 }) }
    }

    fn ctx(&self) -> Result<JsContext, JsDecodeError> {
        let ctx = rquickjs::Context::full(&self.inner.runtime)?;
        let notify = self.inner.notify.clone();
        let timeout = self.inner.timeout;
        Ok(JsContext {
            hand: tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(timeout)).await;
                let s = notify.load(Ordering::Relaxed);
                if !s {
                    notify.store(true, Ordering::Relaxed)
                }
            }),
            ctx,
        })
    }

    pub fn eval(&self, script: &str, data: RawData) -> Result<DecodeData, JsDecodeError> {
        let ctx = self.ctx()?;
        ctx.ctx.with(|ctx| {
            let global = ctx.globals();
            ctx.eval::<(), _>(script)?;
            let decode: Function = global.get(JS_FUNCTION_NAME)
                .map_err(|_| JsDecodeError::Export(format!("most export {}", JS_FUNCTION_NAME)))?;
            let data: DecodeData = decode.call((data,))?;
            Ok(data)
        })
    }

}

#[cfg(test)]
mod tests {
    use crate::decode::{up_data_decode, JsManager, RawData};
    use crate::man::DecodeManager;
    use common_define::Id;

    #[test]
    fn test_decode() {
        let v = vec![
            0x0, 0x1, 0x9, 0x3, 0x1, 0x13, 0x1, 0x22, 0x1, 0x1, 0x1, 0x1, 0x0, 0x1, 0xA, 0x3, 0x1,
            0x13, 0x1, 0x20, 0x4, 0x1, 0x1, 0x1, 0x1,
        ];
        up_data_decode(&v).unwrap();
    }

    #[tokio::test]
    async fn test_js_decode() {
        let rt = JsManager::new();
        let m = DecodeManager::new(rt.clone());
        let s = r#"export function decodeUplink(data) {
  return {
    data: [
      { data: data.bytes[0], id: 0 },
            { data: data.bytes[0], id: 2 }
    ]
  }
}"#;
        let d = m.decode_with_code(s, RawData::new([1, 2, 3])).unwrap();
        println!("{:?}", d);
    }
}
