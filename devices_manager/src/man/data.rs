use crate::decode::DecodeDataDecoded;
use crate::{decode, protocol::lora::payload::LoRaPayload};
use bytes::Bytes;
use common_define::db::Eui;
use common_define::decode::DecodeDataType;
use common_define::ClientId;
use derive_new::new;
use lorawan::parser::DataHeader;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use tracing::{info, instrument, warn};

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Copy,
    Clone,
    Debug,
    strum::AsRefStr,
    strum::EnumString
)]
pub(crate) enum ValueType {
    Array,
    F64,
    F32,
    Bool,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
}

impl From<DecodeDataType> for ValueType {
    fn from(value: DecodeDataType) -> Self {
        match value {
            DecodeDataType::I32 => Self::I32,
            DecodeDataType::F64 => Self::F64,
            DecodeDataType::Bool => Self::Bool,
        }
    }
}

impl TryFrom<u8> for ValueType {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0b0000 => Ok(Self::Array),
            0b0001 => Ok(Self::F64),
            0b0010 => Ok(Self::F32),
            0b0011 => Ok(Self::Bool),
            0b0100 => Ok(Self::I8),
            0b0101 => Ok(Self::U8),
            0b0110 => Ok(Self::I16),
            0b0111 => Ok(Self::U16),
            0b1000 => Ok(Self::I32),
            0b1001 => Ok(Self::U32),
            x => Err(format!("unsupport value type: {}", x)),
        }
    }
}

#[derive(Debug)]
pub(crate) struct DataError {
    pub(crate) msg: String,
}

impl<T> From<T> for DataError
where
    T: ToString,
{
    fn from(value: T) -> Self {
        Self { msg: value.to_string() }
    }
}

pub(crate) enum LoRaPayloadType<'a> {
    Data(LoRaDataFormater<'a>),
    IO(LoRaIOFormater<'a>),
}

pub(crate) struct LoRaDataFormater<'a> {
    payload: &'a [u8],
}

impl<'a> LoRaDataFormater<'a> {
    pub(crate) fn new(data: &'a [u8]) -> LoRaDataFormater<'a> {
        Self { payload: data }
    }
    pub(crate) fn format(&self) -> Result<DecodeDataDecoded, DataError> {
        decode::up_data_decode(self.payload)
    }
}

pub(crate) struct DeviceInfo<'a> {
    data: &'a [u8],
}

impl<'a> DeviceInfo<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Result<DeviceInfo<'a>, DataError> {
        if data.len() == 12 {
            Ok(Self { data })
        } else {
            Err(DataError::from("device info length most 12 byte"))
        }
    }
    pub(crate) fn adr(&self) -> u8 {
        self.data[0]
    }
    pub(crate) fn comfirm(&self) -> bool {
        self.data[1] != 0
    }
    pub(crate) fn dutycycle(&self) -> u16 {
        ((self.data[2] as u16) << 8) | (self.data[3] as u16)
    }
    pub(crate) fn dr(&self) -> u8 {
        self.data[4]
    }
    pub(crate) fn power(&self) -> u8 {
        self.data[5]
    }
    pub(crate) fn time_zone(&self) -> u16 {
        ((self.data[6] as u16) << 8) | (self.data[7] as u16)
    }
    pub(crate) fn battery(&self) -> u16 {
        ((self.data[8] as u16) << 8) | (self.data[9] as u16)
    }
    pub(crate) fn firmware(&self) -> u16 {
        ((self.data[10] as u16) << 8) | (self.data[11] as u16)
    }
}
pub(crate) struct UpdateDeviceInfo<'a> {
    data: &'a [u8],
}

impl<'a> UpdateDeviceInfo<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Result<UpdateDeviceInfo<'a>, DataError> {
        if data.len() == 8 {
            Ok(Self { data })
        } else {
            Err(DataError::from("update device info length most 8 byte"))
        }
    }
    pub(crate) fn adr(&self) -> u8 {
        self.data[0]
    }
    pub(crate) fn comfirm(&self) -> bool {
        self.data[1] != 0
    }
    pub(crate) fn dutycycle(&self) -> u16 {
        ((self.data[2] as u16) << 8) | (self.data[3] as u16)
    }
    pub(crate) fn dr(&self) -> u8 {
        self.data[4]
    }
    pub(crate) fn power(&self) -> u8 {
        self.data[5]
    }
    pub(crate) fn time_zone(&self) -> u16 {
        ((self.data[6] as u16) << 8) | (self.data[7] as u16)
    }
}

pub(crate) struct QueryIO<'a> {
    data: &'a [u8],
}

impl<'a> QueryIO<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Result<QueryIO<'a>, DataError> {
        if data.is_empty() {
            Ok(Self { data })
        } else if data[0] == 0xFF {
            Err(DataError::from("query id error"))
        } else {
            Ok(Self { data })
        }
    }
}
pub(crate) struct UpdateIO<'a> {
    mutil: bool,
    data: &'a [u8],
}

pub(crate) struct IO {
    id: u8,
    value: bool,
}

impl<'a> UpdateIO<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Result<UpdateIO<'a>, DataError> {
        let len = data.len();
        if len < 3 {
            return Err(DataError::from("update io length less than 3"));
        }
        if ((len - 1) & 0b1) == 1 {
            return Err(DataError::from("update io length not match"));
        }
        if data[1] == 0xFF {
            return Err(DataError::from("update io length not match"));
        }
        if len == 3 {
            Ok(Self { mutil: false, data: &data[1..] })
        } else {
            Ok(Self { mutil: true, data: &data[1..] })
        }
    }
    pub(crate) fn state(&self) -> Vec<IO> {
        let len = self.data.len();
        let capacity = len % 2;
        let mut v = Vec::with_capacity(capacity);
        let mut index = 0;
        while len > index {
            v.push(IO { id: self.data[index], value: self.data[index + 1] == 1 });
            index += 2;
        }
        v
    }
}
pub(crate) struct IOTimer<'a> {
    data: &'a [u8],
}

impl<'a> IOTimer<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Result<IOTimer<'a>, DataError> {
        let len = data.len();
        if len != 7 {
            return Err(DataError::from("query timer length 7"));
        }
        Ok(Self { data })
    }
    pub(crate) fn inex(&self) -> u8 {
        self.data[0]
    }
    pub(crate) fn enable(&self) -> bool {
        self.data[1] != 0
    }
    pub(crate) fn io_number(&self) -> u8 {
        self.data[2]
    }
    pub(crate) fn action(&self) -> bool {
        self.data[3] != 0
    }
    pub(crate) fn hour(&self) -> u8 {
        self.data[4]
    }
    pub(crate) fn minute(&self) -> u8 {
        self.data[5]
    }
    pub(crate) fn repeat(&self) -> u8 {
        self.data[6]
    }
}
pub(crate) enum IOComamd<'a> {
    DeviceInfo(DeviceInfo<'a>),
    UpdateDeviceInfo(UpdateDeviceInfo<'a>),
    QueryIO(QueryIO<'a>),
    UpdateIO(UpdateIO<'a>),
    QueryTimer(IOTimer<'a>),
    UpdateTimer(IOTimer<'a>),
}

pub(crate) struct LoRaIOFormater<'a> {
    payload: &'a [u8],
}

impl<'a> LoRaIOFormater<'a> {
    pub(crate) fn new(payload: &'a [u8]) -> LoRaIOFormater<'a> {
        Self { payload }
    }
    pub(crate) fn format(&self) -> Result<IOComamd, DataError> {
        tracing::info!("command {:?}", self.payload);
        let command_type = self.payload.first().ok_or(DataError::from("not found command"))?;
        match command_type {
            0 => Ok(IOComamd::DeviceInfo(DeviceInfo::new(&self.payload[1..])?)),
            1 => Ok(IOComamd::UpdateDeviceInfo(UpdateDeviceInfo::new(&self.payload[1..])?)),
            2 => Ok(IOComamd::QueryIO(QueryIO::new(&self.payload[1..])?)),
            3 => Ok(IOComamd::UpdateIO(UpdateIO::new(&self.payload[1..])?)),
            4 => Ok(IOComamd::QueryTimer(IOTimer::new(&self.payload[1..])?)),
            5 => Ok(IOComamd::UpdateTimer(IOTimer::new(&self.payload[1..])?)),
            _ => Err(DataError::from("Not support io command")),
        }
    }
}

pub(crate) struct LoRaPayloadFormater<'a> {
    header: &'a LoRaPayload,
    payload: &'a [u8],
}

impl<'a> LoRaPayloadFormater<'a> {
    pub(crate) fn new(header: &'a LoRaPayload, payload: &'a [u8]) -> LoRaPayloadFormater<'a> {
        Self { header, payload }
    }
    pub(crate) fn format(&self) -> Result<LoRaPayloadType, DataError> {
        let port = self.header.f_port().ok_or("Not foun port".to_string())?;
        match port {
            2 => {
                let format = LoRaDataFormater::new(self.payload);
                Ok(LoRaPayloadType::Data(format))
            }
            3 => Ok(LoRaPayloadType::IO(LoRaIOFormater::new(self.payload))),
            _ => Err(DataError::from("not support port")),
        }
    }
}

pub(crate) trait CommandBuilder {
    fn data(&self) -> Bytes;
}
pub(crate) struct QueryDeviceBuilder([u8; 1]);

impl QueryDeviceBuilder {
    pub(crate) fn new() -> QueryDeviceBuilder {
        Self([0])
    }
}

impl CommandBuilder for QueryDeviceBuilder {
    fn data(&self) -> Bytes {
        self.0.to_vec().into()
    }
}

pub(crate) struct UpdateDeviceInfoBuilder([u8; 10]);

impl UpdateDeviceInfoBuilder {
    pub(crate) fn new() -> Self {
        Self([1, 0, 0, 0, 0, 0, 0, 0, 0, 0])
    }

    pub(crate) fn set_adr(mut self, adr: u8) -> Self {
        self.0[1] = adr;
        self
    }
    pub(crate) fn set_confirm(mut self, confirm: bool) -> Self {
        self.0[2] = if confirm { 1 } else { 0 };
        self
    }
    pub(crate) fn set_dutycycle(mut self, dutycycle: u16) -> Self {
        self.0[3] = (dutycycle >> 8) as u8;
        self.0[4] = dutycycle as u8;
        self
    }
    pub(crate) fn set_dr(mut self, dr: u8) -> Self {
        self.0[5] = dr;
        self
    }
    pub(crate) fn set_power(mut self, power: u16) -> Self {
        self.0[6] = (power >> 8) as u8;
        self.0[7] = power as u8;
        self
    }
    pub(crate) fn set_time_zone(mut self, time_zone: u16) -> Self {
        self.0[8] = (time_zone >> 8) as u8;
        self.0[9] = time_zone as u8;
        self
    }
}

impl CommandBuilder for UpdateDeviceInfoBuilder {
    fn data(&self) -> Bytes {
        self.0.to_vec().into()
    }
}

pub(crate) struct QueryIOBuilder([u8; 1]);
impl QueryIOBuilder {
    pub(crate) fn new() -> Self {
        Self([2])
    }
}

impl CommandBuilder for QueryIOBuilder {
    fn data(&self) -> Bytes {
        self.0.to_vec().into()
    }
}

pub(crate) struct UpdateIOBuilder {
    mutil: bool,
    data: [u8; 6],
}

impl UpdateIOBuilder {
    pub(crate) fn new(mutil: bool) -> Self {
        let mut data = [0; 6];
        data[0] = 3;
        data[1] = if mutil { 1 } else { 0 };
        Self { mutil, data }
    }

    pub(crate) fn set_io(mut self, pin: u8, value: bool) -> Self {
        assert!(pin < 16);
        if self.mutil {
            if pin < 8 {
                self.data[3] |= 1 << pin;
                if value {
                    self.data[5] |= 1 << pin;
                } else {
                    self.data[5] &= !(1 << pin);
                }
            } else {
                let pin = pin - 8;
                self.data[2] |= 1 << pin;
                if value {
                    self.data[4] |= 1 << pin;
                } else {
                    self.data[4] &= !(1 << pin);
                }
            }
        } else {
            self.data[2] = pin;
            self.data[3] = if value { 1 } else { 0 };
        }
        self
    }
}

impl CommandBuilder for UpdateIOBuilder {
    fn data(&self) -> Bytes {
        if self.mutil {
            self.data.to_vec().into()
        } else {
            self.data[0..4].to_vec().into()
        }
    }
}
pub(crate) struct QueryTimerBuilder([u8; 2]);
impl QueryTimerBuilder {
    pub(crate) fn new(num: u8) -> Self {
        Self([4, num])
    }
}

impl CommandBuilder for QueryTimerBuilder {
    fn data(&self) -> Bytes {
        self.0.to_vec().into()
    }
}
pub(crate) struct UpdateTimerBuilder([u8; 8]);
impl UpdateTimerBuilder {
    pub(crate) fn new() -> Self {
        Self([5, 0, 0, 0, 0, 0, 0, 0])
    }
    pub(crate) fn set_num(mut self, num: u8) -> Self {
        self.0[1] = num;
        self
    }
    pub(crate) fn set_enable(mut self, enable: bool) -> Self {
        self.0[2] = if enable { 1 } else { 0 };
        self
    }
    pub(crate) fn set_pin(mut self, pin: u8) -> Self {
        self.0[3] = pin;
        self
    }
    pub(crate) fn set_action(mut self, action: bool) -> Self {
        self.0[4] = if action { 1 } else { 0 };
        self
    }
    pub(crate) fn set_hour(mut self, hour: u8) -> Self {
        self.0[5] = hour;
        self
    }
    pub(crate) fn set_minute(mut self, minute: u8) -> Self {
        self.0[6] = minute;
        self
    }
    pub(crate) fn set_repeat(mut self, repeat: u8) -> Self {
        self.0[7] = repeat;
        self
    }
}

impl CommandBuilder for UpdateTimerBuilder {
    fn data(&self) -> Bytes {
        self.0.to_vec().into()
    }
}

#[derive(Clone)]
pub struct DownloadData {
    pub port: u8,
    pub up_count: Option<u32>,
    pub bytes: Bytes,
    pub id: u32,
}

impl DownloadData {
    pub(crate) fn new_io<T: CommandBuilder>(command: T) -> Self {
        let bytes = command.data();
        Self { id: 0, up_count: None, port: 3, bytes }
    }
    pub(crate) fn new_data<D: Into<Bytes>>(data: D) -> Self {
        let bytes = data.into();
        Self { id: 0, up_count: None, bytes, port: 2 }
    }
    pub(crate) fn new_data_with_id<D: Into<Bytes>>(
        data: D,
        id: u32,
        port: u8,
    ) -> Self {
        let bytes = data.into();
        Self { id, up_count: None, bytes, port }
    }
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, new)]
struct DownloadDataWrap {
    data: DownloadData,
    #[new(default)]
    count: u32,
    down_id: u64,
}

#[derive(Default)]
pub struct DownloadDataCache {
    data: Mutex<HashMap<Eui, VecDeque<DownloadDataWrap>>>,
}

impl DownloadDataCache {
    pub fn insert(&self, eui: Eui, message: DownloadData) -> u64 {
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut d = self.data.lock().unwrap();
        if let Some(d) = d.get_mut(&eui) {
            d.push_back(DownloadDataWrap::new(message, counter));
            return counter;
        }
        let mut v = VecDeque::new();
        v.push_back(DownloadDataWrap::new(message, counter));
        d.insert(eui, v);
        counter
    }

    pub fn repetition_task(&self, eui: Eui, counter: u64) -> bool {
        let d = self.data.lock().unwrap();
        d.get(&eui).and_then(|v| v.front().map(|f| f.down_id == counter)).unwrap_or(false)
    }

    pub fn has(&self, eui: Eui) -> bool {
        let d = self.data.lock().unwrap();
        d.get(&eui).and_then(|v| v.front()).is_some()
    }
    pub fn pop(&self, eui: Eui) -> Option<DownloadData> {
        let mut d = self.data.lock().unwrap();
        d.get_mut(&eui).and_then(|v| v.pop_front().map(|e| e.data))
    }

    #[instrument(skip(self))]
    pub fn commit(&self, eui: Eui) {
        let mut d = self.data.lock().unwrap();
        if let Some(v) = d.get_mut(&eui) {
            match v.pop_front() {
                None => {
                    warn!("commit not found data");
                }
                Some(o) => {
                    info!("commit id: {}, port: {}", o.data.id, o.data.port);
                }
            }
        }
    }
}


#[derive(Default)]
pub struct DownloadResponse {
    data: Mutex<HashMap<Eui, tokio::sync::oneshot::Sender<(Vec<u8>, u8)>>>,
}

impl DownloadResponse {
    pub fn has(&self, eui: Eui) -> bool {
        let d = self.data.lock().unwrap();
        d.get(&eui).is_some()
    }

    pub fn get(&self, eui: Eui) -> Option<tokio::sync::oneshot::Sender<(Vec<u8>, u8)>> {
        let mut d = self.data.lock().unwrap();
        d.remove(&eui)
    }

    pub fn add(&self, eui: Eui) -> Option<tokio::sync::oneshot::Receiver<(Vec<u8>, u8)>> {
        if self.has(eui) {
            return None
        }
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut d = self.data.lock().unwrap();
        d.insert(eui, tx);
        Some(rx)
    }
}