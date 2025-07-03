use crate::protocol::snap::CustomError;
use aes::cipher::KeyIvInit;
use aes::cipher::StreamCipher;
use aes::Aes128;
use base64::Engine;
use cmac::{Cmac, Mac};
use common_define::db::{Eui, Key};

fn check_version(version: u8) -> bool {
    version == 1
}

fn is_uplink(uplink: u8) -> bool {
    uplink == 0
}

fn is_downlink(downlink: u8) -> bool {
    downlink == 1
}

fn get_data_start(data: &[u8]) -> Result<usize, CustomError> {
    let len = data.len() as u32;
    if len < 19 {
        return Err(CustomError::Format(format!("require most 19 bytes, but found {}", len)));
    }
    if !check_version(data[0]) {
        return Err(CustomError::Format(format!("not support version {}", data[0])));
    }
    let payload_len = data[14] as u32;
    match payload_len {
        payload_len if payload_len < 253 => {
            if 18 + 1 + payload_len == len {
                Ok(15)
            } else {
                Err(CustomError::Format(format!(
                    "payload length is {}, but data length is {}",
                    payload_len, len
                )))
            }
        }
        253 => {
            let b2_payload_len = data[15] as u32 | ((data[16] as u32) << 8);
            if 18 + 3 + b2_payload_len == len {
                Ok(17)
            } else {
                Err(CustomError::Format(format!(
                    "payload length is {}, but data length is {}",
                    payload_len, len
                )))
            }
        }
        254 => {
            let b4_payload_len = data[15] as u32
                | ((data[16] as u32) << 8)
                | ((data[17] as u32) << 16)
                | ((data[18] as u32) << 24);
            if 18 + 5 + b4_payload_len == len {
                Ok(19)
            } else {
                Err(CustomError::Format(format!(
                    "payload length is {}, but data length is {}",
                    payload_len, len
                )))
            }
        }
        _ => Err(CustomError::Format(format!(
            "payload length is {}, but data length is {}",
            payload_len, len
        ))),
    }
}

// fn encrypt_frm_data_payload(
//     phy_payload: &mut [u8],
//     iv: u32,
//     aes_enc: &dyn keys::Encrypter,
// ) {
//     let len = end - start;
//
//     let mut a = [0u8; 16];
//     generate_helper_block(phy_payload, 0x01, fcnt, &mut a[..]);
//
//     let mut s = [0u8; 16];
//     let s_block = GenericArray::from_mut_slice(&mut s[..]);
//
//     let mut ctr = 1;
//     for i in 0..len {
//         let j = i & 0x0f;
//         if j == 0 {
//             a[15] = ctr;
//             ctr += 1;
//             s_block.copy_from_slice(&a);
//             aes_enc.encrypt_block(s_block);
//         }
//         phy_payload[start + i] ^= s_block[j]
//     }
// }

#[derive(Debug)]
pub struct UpData {
    eui: Eui,
    enc: bool,
    p_type: u8,
    port: u8,
    option: u8,
    count: u16,
    payload_start: usize,
    payload: Vec<u8>,
    bytes: Vec<u8>,
    mic: [u8; 4],
}

impl UpData {
    const UP_ACK: u8 = 0x80;
    pub fn new(bytes: Vec<u8>) -> Result<Self, CustomError> {
        let payload_start = get_data_start(&bytes)?;
        let eui = Eui::from_be_bytes(&bytes[1..9])
            .ok_or(CustomError::Format("eui found error".to_string()))?;
        let p_type = bytes[9];
        let port = bytes[10];
        let option = bytes[11];
        let count = bytes[12] as u16 | ((bytes[13] as u16) << 8);
        let payload_end = bytes.len() - 4;
        let mut payload = Vec::with_capacity(payload_end - payload_start);
        payload.extend_from_slice(&bytes[payload_start..payload_end]);
        let mut mic = [0; 4];
        mic.copy_from_slice(&bytes[payload_end..]);
        Ok(Self { eui, enc: true, p_type, port, option, count, payload_start, payload, bytes, mic })
    }

    pub fn counter(&self) -> u16 {
        self.count
    }
    pub fn ack(&self) -> bool {
        self.option & Self::UP_ACK == Self::UP_ACK
    }

    pub fn is_enc(&self) -> bool {
        self.enc
    }
    pub fn check_mic(&self, key: &Key) -> Result<bool, CustomError> {
        let mut black_a = [0; 16];
        let payload_with_header_len = self.bytes.len() - 4;
        black_a[0..8].copy_from_slice(&self.bytes[1..9]);
        black_a[8] = self.bytes[12];
        black_a[9] = self.bytes[13];
        black_a[12] = payload_with_header_len as u8;
        black_a[13] = (payload_with_header_len >> 8) as u8;
        black_a[15] = 0x01;
        let mut mac = Cmac::<Aes128>::new_from_slice(&key.0 .0).map_err(|_| CustomError::Key)?;
        Mac::update(&mut mac, &black_a);
        Mac::update(&mut mac, &self.bytes[0..payload_with_header_len]);
        let result = Mac::finalize(mac);
        let buf = result.into_bytes();
        Ok(buf[0] == self.mic[0]
            && buf[1] == self.mic[1]
            && buf[2] == self.mic[2]
            && buf[3] == self.mic[3])
    }

    pub fn decode_payload(&mut self, key: &Key) -> Result<&[u8], CustomError> {
        if self.check_mic(key)? {
            let mut iv = [0; 16];
            let payload_len = self.payload.len();
            iv[0..8].copy_from_slice(&self.bytes[1..9]);
            iv[8] = self.bytes[12];
            iv[9] = self.bytes[13];
            iv[12] = payload_len as u8;
            iv[13] = (payload_len >> 8) as u8;
            iv[14] = 0x01;
            let mut cipher = ctr::Ctr128BE::<Aes128>::new(&key.0 .0.into(), &iv.into());
            cipher.apply_keystream(&mut self.payload);
            Ok(&self.payload)
        } else {
            Err(CustomError::MIC)
        }
    }

    pub fn eui(&self) -> Eui {
        self.eui
    }
}

pub struct DownloadData {
    eui: Eui,
    port: u8,
    option: u8,
    count: u16,
}

impl Default for DownloadData {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloadData {
    pub fn new() -> Self {
        Self { eui: Default::default(), port: 0, option: 0, count: 0 }
    }
    pub fn new_with_eui(eui: Eui) -> Self {
        Self { eui, port: 0, option: 0, count: 0 }
    }
    pub fn set_ack(mut self) -> Self {
        self.option |= 0x40;
        self
    }
    pub fn set_eui(mut self, eui: Eui) -> Self {
        self.eui = eui;
        self
    }
    pub fn set_counter(mut self, count: u16) -> Self {
        self.count = count;
        self
    }
    pub fn set_port(mut self, port: u8) -> Self {
        self.port = port;
        self
    }
    pub fn encode_payload(&self, payload: &[u8], key: &Key) -> Result<String, CustomError> {
        let payload_len = payload.len();
        let header_and_mic_len = match payload_len {
            download_len if download_len < 253 => 19,
            253 => 21,
            254 => 23,
            _ => {
                return Err(CustomError::Format(format!("data length is {} to much", payload_len)))
            }
        };
        let mut buf = Vec::with_capacity(header_and_mic_len + payload_len);
        buf.push(0x01);
        buf.extend_from_slice(&self.eui.to_be_bytes());
        buf.push(0x01);
        buf.push(self.port);
        buf.push(self.option);
        buf.push(self.count as u8);
        buf.push((self.count >> 8) as u8);
        if header_and_mic_len == 19 {
            buf.push(payload_len as u8);
        }
        if header_and_mic_len == 21 {
            buf.push(payload_len as u8);
            buf.push((payload_len >> 8) as u8);
        }
        if header_and_mic_len == 23 {
            buf.push(payload_len as u8);
            buf.push((payload_len >> 8) as u8);
            buf.push((payload_len >> 16) as u8);
            buf.push((payload_len >> 24) as u8);
        }
        buf.extend_from_slice(payload);
        self.encode(&mut buf[header_and_mic_len - 4..], key)?;
        let mic = self.mic(&buf, key)?;
        buf.extend_from_slice(&mic);
        Ok(base64::engine::general_purpose::STANDARD.encode(buf))
    }

    fn encode(&self, payload: &mut [u8], key: &Key) -> Result<(), CustomError> {
        let mut iv = [0; 16];
        let payload_len = payload.len();
        iv[0..8].copy_from_slice(&self.eui.to_bytes());
        iv[8] = self.count as u8;
        iv[9] = (self.count >> 8) as u8;
        iv[12] = payload_len as u8;
        iv[13] = (payload_len >> 8) as u8;
        iv[14] = 0x00;
        let mut cipher = ctr::Ctr128BE::<Aes128>::new(&key.0 .0.into(), &iv.into());
        cipher.apply_keystream(payload);
        Ok(())
    }

    fn mic(&self, payload_with_header: &[u8], key: &Key) -> Result<[u8; 4], CustomError> {
        let mut mic = [0; 4];
        let mut black_a = [0; 16];
        let payload_with_header_len = payload_with_header.len();
        black_a[0..8].copy_from_slice(&payload_with_header[1..9]);
        black_a[8] = payload_with_header[12];
        black_a[9] = payload_with_header[13];
        black_a[12] = payload_with_header_len as u8;
        black_a[13] = (payload_with_header_len >> 8) as u8;
        black_a[15] = 0x00;
        let mut mac = Cmac::<Aes128>::new_from_slice(&key.0 .0).map_err(|_| CustomError::Key)?;
        Mac::update(&mut mac, &black_a);
        Mac::update(&mut mac, payload_with_header);
        let result = Mac::finalize(mac).into_bytes();
        mic.copy_from_slice(&result[0..4]);
        Ok(mic)
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::snap::payload::{DownloadData, UpData};
    use base64::Engine;
    use common_define::db::{Eui, Key};
    use std::str::FromStr;

    #[test]
    fn test_decode() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode("ARIiIiIiIiIiAAAAAQAd8JBLbxv+9wUlTAQTlVq5ZlQBfZjO4yiC0xDR9/nxotcb")
            .unwrap();
        let key = Key::from_str("12345678888888888888888888888888").unwrap();
        let mut up = UpData::new(bytes).unwrap();
        let s = up.decode_payload(&key).unwrap();
        let s = std::str::from_utf8(s).unwrap();
        assert_eq!(s, "1234567890abcdefghudfvertexcv");
    }
    #[test]
    fn test_encode() {
        let key = Key::from_str("12345678888888888888888888888888").unwrap();
        let payload = b"1234567890abcdefghudfvertexcv";
        let down = DownloadData::new().set_eui(Eui::from_str("2222222222222212").unwrap());
        let s = down.encode_payload(payload, &key).unwrap();
        println!("{}", s);
    }
}
