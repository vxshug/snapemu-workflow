use crate::man::data::{DataError, DownloadData};
use crate::{man::data::CommandBuilder, service::lorawan_node::PushData};
use crate::{DeviceError, DeviceResult};
use base64::Engine;
use common_define::db::LoRaAddr;
use common_define::lora::LoRaRegion;
use common_define::lorawan_bridge::{DownStream, UpMode, RXPK, TXPK};
use device_info::lorawan::{GatewayInfo, NodeInfo};
use lorawan::keys::AES128;
use lorawan::maccommands::SerializableMacCommand;

pub(crate) struct RespDataBuilder<'a> {
    node: &'a NodeInfo,
    meta: &'a PushData,
}

impl<'a> RespDataBuilder<'a> {
    pub fn new(node: &'a NodeInfo, meta: &'a PushData) -> Self {
        Self { node, meta }
    }
    pub(crate) fn build_with_task(
        &self,
        down: &DownloadData,
        token: u16,
        version: u8,
    ) -> DeviceResult<DownStream> {
        let data = &down.bytes;
        self.build(data, &[], Some(down.port))
    }
    pub(crate) fn build_ack(
        &self,
        mac: &[&dyn SerializableMacCommand],
    ) -> DeviceResult<DownStream> {
        self.build("", mac, None)
    }
    pub(crate) fn build<P: AsRef<[u8]>>(
        &self,
        data: P,
        mac: &[&dyn SerializableMacCommand],
        port: Option<u8>,
    ) -> DeviceResult<DownStream> {
        let data = data.as_ref();
        let mut phy = lorawan::creator::DataPayloadCreator::new();
        phy.set_confirmed(false)
            .set_uplink(false)
            .set_dev_addr(&self.node.dev_addr.to_bytes())
            .set_fctrl(&lorawan::parser::FCtrl::new(0xA0, false))
            .set_fcnt(self.node.down_count);
        if let Some(port) = port {
            phy.set_f_port(port);
        }
        let r = phy
            .build(data, mac, &self.node.nwk_skey, &self.node.app_skey)
            .map_err(DataError::from)?;
        let len = r.len();
        let data = base64::engine::general_purpose::STANDARD.encode(r);
        let txpk = self.calc_args(data, Some(len as u32))?;
        let resp = DownStream::new(txpk);
        Ok(resp)
    }
    pub fn calc_args(&self, data: String, size: Option<u32>) -> DeviceResult<TXPK> {
        let imme = false;
        let tmst = self.calc_tmst().into();
        let freq = self.calc_freq()?;
        let _time = String::new();
        let rfch = 0;
        let powe = self.calc_powe().into();
        let modu = UpMode::LORA;
        let ipol = true;
        let _snr = 0;
        let _lqi = 0;
        let datr = self.calc_datr();
        let codr = self.calc_codr().into();
        let ncrc = true.into();
        Ok(TXPK { imme, tmst, freq, rfch, powe, modu, datr, codr, ipol, size, data, ncrc })
    }

    fn calc_datr(&self) -> Option<String> {
        let (sf, bw) = {
            let s = &self.meta.pk.datr;

            let (sf_num, bw) = s.split_once("BW")?;
            let sf: i32 = sf_num[2..].parse().ok()?;
            let bw: i32 = bw.parse().ok()?;
            (sf, bw)
        };
        Some(match self.node.region {
            LoRaRegion::EU868 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::US915 => format!("SF{}BW{}", sf, 500),
            LoRaRegion::CN779 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::EU433 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::AU915 => format!("SF{}BW{}", sf, 500),
            LoRaRegion::CN470 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::AS923_1 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::AS923_2 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::AS923_3 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::KR920 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::IN865 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::RU864 => format!("SF{}BW{}", sf, bw),
        })
    }
    fn calc_tmst(&self) -> u32 {
        self.meta.pk.tmst.wrapping_add((self.node.rx1_delay as u32) * 1000000)
    }
    fn calc_freq(&self) -> DeviceResult<f32> {
        let freq = DownFreqDrBand::new(self.node.region, &self.meta.pk)?;
        Ok(freq.freq)
    }
    fn calc_powe(&self) -> i32 {
        match self.node.region {
            common_define::lora::LoRaRegion::EU868 => 14,
            common_define::lora::LoRaRegion::US915 => 14,
            common_define::lora::LoRaRegion::CN779 => 17,
            common_define::lora::LoRaRegion::EU433 => 14,
            common_define::lora::LoRaRegion::AU915 => 14,
            common_define::lora::LoRaRegion::CN470 => 17,
            common_define::lora::LoRaRegion::AS923_1 => 14,
            common_define::lora::LoRaRegion::AS923_2 => 14,
            common_define::lora::LoRaRegion::AS923_3 => 14,
            common_define::lora::LoRaRegion::KR920 => 17,
            common_define::lora::LoRaRegion::IN865 => 17,
            common_define::lora::LoRaRegion::RU864 => 17,
        }
    }
    fn calc_codr(&self) -> String {
        let s = match self.node.region {
            common_define::lora::LoRaRegion::EU868 => "4/5",
            common_define::lora::LoRaRegion::US915 => "4/5",
            common_define::lora::LoRaRegion::CN779 => "4/5",
            common_define::lora::LoRaRegion::EU433 => "4/5",
            common_define::lora::LoRaRegion::AU915 => "4/5",
            common_define::lora::LoRaRegion::CN470 => "4/5",
            common_define::lora::LoRaRegion::AS923_1 => "4/5",
            common_define::lora::LoRaRegion::AS923_2 => "4/5",
            common_define::lora::LoRaRegion::AS923_3 => "4/5",
            common_define::lora::LoRaRegion::KR920 => "4/5",
            common_define::lora::LoRaRegion::IN865 => "4/5",
            common_define::lora::LoRaRegion::RU864 => "4/5",
        };
        s.into()
    }
}

pub(crate) struct RespDataClassCBuilder<'a> {
    node: &'a NodeInfo,
    gate: &'a GatewayInfo,
}

impl<'a> RespDataClassCBuilder<'a> {
    pub fn new(node: &'a NodeInfo, gate: &'a GatewayInfo) -> Self {
        Self { node, gate }
    }
    pub(crate) fn build_io<T: CommandBuilder>(
        &self,
        command: T,
        _gateway: uuid::Uuid,
        token: u16,
    ) -> DeviceResult<DownStream> {
        let command = command.data();
        self.build(command, 3, token)
    }
    pub(crate) fn build_with_task(
        &self,
        task: &DownloadData,
        token: u16,
    ) -> DeviceResult<DownStream> {
        self.build(task.bytes.as_ref(), task.port, token)
    }
    pub(crate) fn build_data<D: AsRef<[u8]>>(
        &self,
        data: D,
        _gateway: uuid::Uuid,
        token: u16,
    ) -> DeviceResult<DownStream> {
        self.build(data, 2, token)
    }
    pub(crate) fn build<P: AsRef<[u8]>>(
        &self,
        data: P,
        port: u8,
        token: u16,
    ) -> DeviceResult<DownStream> {
        let data = data.as_ref();
        let mut phy = lorawan::creator::DataPayloadCreator::new();
        phy.set_confirmed(false)
            .set_uplink(false)
            .set_f_port(port)
            .set_dev_addr(&self.node.dev_addr.to_bytes())
            .set_fctrl(&lorawan::parser::FCtrl::new(0xA0, false))
            .set_fcnt(self.node.down_count);
        let r = phy
            .build(data, &[], &self.node.nwk_skey, &self.node.app_skey)
            .map_err(DataError::from)?;
        let len = r.len();
        let data = base64::engine::general_purpose::STANDARD.encode(r);
        let txpk = self.calc_args(data, Some(len as u32))?;
        let resp = DownStream::new(txpk);
        Ok(resp)
    }
    pub fn calc_args(&self, data: String, size: Option<u32>) -> DeviceResult<TXPK> {
        let imme = true;
        let tmst = self.calc_tmst();
        let freq = freq_calc(self.node.rx2_freq);
        let rfch = 0;
        let powe = 17.into();
        let modu = UpMode::LORA;
        let ipol = true;
        let _snr = true;
        let _lqi = true;
        let datr = datr_calc(self.node.des_rx2_dr, 125).into();
        let codr = Some("4/5".into());
        let ncrc = true.into();
        Ok(TXPK { imme, tmst, freq, rfch, powe, modu, datr, codr, ipol, size, data, ncrc })
    }

    fn calc_tmst(&self) -> Option<u32> {
        // Some(self.gate.tmst.wrapping_add(500000))
        None
    }
}

pub(crate) struct JoinRespDataBuilder<'a> {
    node: &'a NodeInfo,
    meta: &'a PushData,
}

impl<'a> JoinRespDataBuilder<'a> {
    const JOIN_ACCEPT_DELAY1: u32 = 5;
    pub fn new(node: &'a NodeInfo, meta: &'a PushData) -> Self {
        Self { node, meta }
    }
    pub(crate) fn build(
        &self,
        addr: LoRaAddr,
        app_nonce: u32,
        net_id: u32,
        app_key: &AES128,
    ) -> DeviceResult<DownStream> {
        let mut build = super::join_accept::AcceptJoin::new();
        let join_data = build
            .set_dev_addr(addr.into())
            .set_app_nonce(app_nonce)
            .set_rx_delay(self.node.rx1_delay as u8)
            .set_net_id(net_id)
            .build(app_key)?;
        let len = join_data.len();
        let data = base64::engine::general_purpose::STANDARD.encode(join_data);
        let txpk = self.calc_args(data, Some(len as u32))?;
        let resp = DownStream::new(txpk);
        Ok(resp)
    }
    pub fn calc_args(&self, data: String, size: Option<u32>) -> DeviceResult<TXPK> {
        let imme = false;
        let tmst = self.calc_tmst().into();
        let freq = self.calc_freq()?;
        let rfch = 0;
        let powe = self.calc_powe().into();
        let modu = UpMode::LORA;
        let ipol = true;
        let datr = self.calc_datr();
        let codr = self.calc_codr().into();
        let ncrc = true.into();
        Ok(TXPK { imme, tmst, freq, rfch, powe, modu, datr, codr, ipol, size, data, ncrc })
    }

    fn calc_datr(&self) -> Option<String> {
        let (sf, bw) = {
            let s = &self.meta.pk.datr;
            let (sf_num, bw) = s.split_once("BW")?;
            let sf: i32 = sf_num[2..].parse().ok()?;
            let bw: i32 = bw.parse().ok()?;
            (sf, bw)
        };
        Some(match self.node.region {
            LoRaRegion::EU868 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::US915 => format!("SF{}BW{}", sf, 500),
            LoRaRegion::CN779 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::EU433 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::AU915 => format!("SF{}BW{}", sf, 500),
            LoRaRegion::CN470 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::AS923_1 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::AS923_2 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::AS923_3 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::KR920 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::IN865 => format!("SF{}BW{}", sf, bw),
            LoRaRegion::RU864 => format!("SF{}BW{}", sf, bw),
        })
    }
    fn calc_tmst(&self) -> u32 {
        self.meta.pk.tmst.wrapping_add(Self::JOIN_ACCEPT_DELAY1 * 1000000)
    }
    fn calc_freq(&self) -> DeviceResult<f32> {
        let freq = DownFreqDrBand::new(self.node.region, &self.meta.pk)?;
        Ok(freq.freq)
    }
    fn calc_powe(&self) -> i32 {
        match self.node.region {
            common_define::lora::LoRaRegion::EU868 => 14,
            common_define::lora::LoRaRegion::US915 => 14,
            common_define::lora::LoRaRegion::CN779 => 17,
            common_define::lora::LoRaRegion::EU433 => 14,
            common_define::lora::LoRaRegion::AU915 => 14,
            common_define::lora::LoRaRegion::CN470 => 17,
            common_define::lora::LoRaRegion::AS923_1 => 14,
            common_define::lora::LoRaRegion::AS923_2 => 14,
            common_define::lora::LoRaRegion::AS923_3 => 14,
            common_define::lora::LoRaRegion::KR920 => 17,
            common_define::lora::LoRaRegion::IN865 => 17,
            common_define::lora::LoRaRegion::RU864 => 17,
        }
    }
    fn calc_codr(&self) -> String {
        let s = match self.node.region {
            common_define::lora::LoRaRegion::EU868 => "4/5",
            common_define::lora::LoRaRegion::US915 => "4/5",
            common_define::lora::LoRaRegion::CN779 => "4/5",
            common_define::lora::LoRaRegion::EU433 => "4/5",
            common_define::lora::LoRaRegion::AU915 => "4/5",
            common_define::lora::LoRaRegion::CN470 => "4/5",
            common_define::lora::LoRaRegion::AS923_1 => "4/5",
            common_define::lora::LoRaRegion::AS923_2 => "4/5",
            common_define::lora::LoRaRegion::AS923_3 => "4/5",
            common_define::lora::LoRaRegion::KR920 => "4/5",
            common_define::lora::LoRaRegion::IN865 => "4/5",
            common_define::lora::LoRaRegion::RU864 => "4/5",
        };
        s.into()
    }
}

pub(crate) struct DownFreqDrBand {
    pub(crate) freq: f32,
}

impl DownFreqDrBand {
    const EU868_START: f32 = 868.10;
    const EU868_END: f32 = 868.50;
    const EU868_STEP: f32 = 0.2;

    const US915_START_1: f32 = 902.3;
    const US915_END_1: f32 = 914.9;
    const US915_STEP_1: f32 = 0.2;

    const US915_START_2: f32 = 903.0;
    const US915_END_2: f32 = 914.2;
    const US915_STEP_2: f32 = 1.6;

    const CN779_START_1: f32 = 779.5;
    const CN779_END_1: f32 = 779.9;
    const CN779_STEP_1: f32 = 0.2;
    const CN779_START_2: f32 = 780.5;
    const CN779_END_2: f32 = 780.9;
    const CN779_STEP_2: f32 = 0.2;

    const EU443_START: f32 = 443.175;
    const EU443_END: f32 = 443.575;
    const EU443_STEP: f32 = 0.2;

    const AU915_START_1: f32 = 915.2;
    const AU915_END_1: f32 = 927.8;
    const AU915_STEP_1: f32 = 0.2;
    const AU915_START_2: f32 = 915.9;
    const AU915_END_2: f32 = 927.1;
    const AU915_STEP_2: f32 = 1.6;

    const CN470_START: f32 = 470.3;
    const CN470_END: f32 = 489.3;
    const CN470_STEP: f32 = 0.2;

    const AS923_START: f32 = 923.2;
    const AS923_END: f32 = 923.4;
    const AS923_STEP: f32 = 0.2;

    const KR920_START: f32 = 922.1;
    const KR920_END: f32 = 923.3;
    const KR920_STEP: f32 = 0.2;

    const IN865_1: f32 = 865.0625;
    const IN8650_2: f32 = 865.4025;
    const IN8650_3: f32 = 865.985;

    const RU864_1: f32 = 868.9;
    const RU864_2: f32 = 869.1;
    pub(crate) fn new(region: LoRaRegion, rx: &RXPK) -> DeviceResult<Self> {
        let chan = Self::freq_chan(rx.freq, region)?;
        let freq = Self::tx_freq(region, rx.freq, chan);
        Ok(Self { freq })
    }

    fn tx_freq(region: LoRaRegion, freq: f32, chan: u8) -> f32 {
        match region {
            LoRaRegion::EU868 => freq,
            LoRaRegion::US915 => {
                let down_chan = (chan % 8) as f32;
                923.3 + (down_chan * 0.6)
            }
            LoRaRegion::CN779 => freq,
            LoRaRegion::EU433 => freq,
            LoRaRegion::AU915 => {
                let down_chan = (chan % 8) as f32;
                923.3 + (down_chan * 0.6)
            }
            LoRaRegion::CN470 => {
                let down_chan = (chan % 48) as f32;
                500.3 + (down_chan * 0.2)
            }
            LoRaRegion::AS923_1 => freq,
            LoRaRegion::AS923_2 => freq,
            LoRaRegion::AS923_3 => freq,
            LoRaRegion::KR920 => freq,
            LoRaRegion::IN865 => freq,
            LoRaRegion::RU864 => freq,
        }
    }

    fn freq_chan(freq: f32, region: LoRaRegion) -> DeviceResult<u8> {
        match region {
            LoRaRegion::EU868 => Ok(0),
            LoRaRegion::US915 => {
                Self::_calc_chan(freq, Self::US915_START_1, Self::US915_END_1, Self::US915_STEP_1)
                    .or(Self::_calc_chan(
                        freq,
                        Self::US915_START_2,
                        Self::US915_END_2,
                        Self::US915_STEP_2,
                    )
                    .map(|c| c + 64))
            }
            LoRaRegion::CN779 => {
                Self::_calc_chan(freq, Self::CN779_START_1, Self::CN779_END_1, Self::CN779_STEP_1)
                    .or(Self::_calc_chan(
                        freq,
                        Self::CN779_START_2,
                        Self::CN779_END_2,
                        Self::CN779_STEP_2,
                    )
                    .map(|c| c + 3))
            }
            LoRaRegion::EU433 => {
                Self::_calc_chan(freq, Self::EU443_START, Self::EU443_END, Self::EU443_STEP)
            }
            LoRaRegion::AU915 => {
                Self::_calc_chan(freq, Self::AU915_START_1, Self::AU915_END_1, Self::AU915_STEP_1)
                    .or(Self::_calc_chan(
                        freq,
                        Self::AU915_START_2,
                        Self::AU915_END_2,
                        Self::AU915_STEP_2,
                    )
                    .map(|c| c + 64))
            }
            LoRaRegion::CN470 => {
                Self::_calc_chan(freq, Self::CN470_START, Self::CN470_END, Self::CN470_STEP)
            }
            LoRaRegion::AS923_1 => Ok(0),

            LoRaRegion::KR920 => {
                Self::_calc_chan(freq, Self::KR920_START, Self::KR920_END, Self::KR920_STEP)
            }
            LoRaRegion::AS923_2 => Ok(0),
            LoRaRegion::AS923_3 => Ok(0),
            LoRaRegion::IN865 => {
                if (freq - Self::IN865_1).abs() < 0.00001 {
                    Ok(0)
                } else if (freq - Self::IN8650_2).abs() < 0.00001 {
                    Ok(1)
                } else if (freq - Self::IN8650_3).abs() < 0.00001 {
                    Ok(2)
                } else {
                    Err(DeviceError::Warn(format!("IN865 not support frq: {} ", freq)))
                }
            }
            LoRaRegion::RU864 => {
                if (freq - Self::RU864_1).abs() < 0.00001 {
                    Ok(0)
                } else if (freq - Self::RU864_2).abs() < 0.00001 {
                    Ok(1)
                } else {
                    Err(DeviceError::Warn(format!("RU864 not support frq: {} ", freq)))
                }
            }
        }
    }

    fn _calc_chan(freq: f32, left: f32, right: f32, setup: f32) -> DeviceResult<u8> {
        if (freq < (right + 0.001)) && ((left - 0.001) < freq) {
            Ok(((freq - left) / setup + 0.001) as u8)
        } else {
            Err(DeviceError::Data(format!("freq: {}, most {} between {}", freq, left, right)))
        }
    }
}

fn freq_calc(freq: i32) -> f32 {
    (freq as f32) / 10000.0
}
fn datr_calc(dr: i16, board: i32) -> String {
    format!("SF{}BW{}", 12 - dr, board)
}
