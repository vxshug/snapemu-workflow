pub fn u8_to_token(u: &[u8]) -> Option<u16> {
    if u.len() < 2 {
        return None;
    }
    let mut token = 0;
    token += (u[0] as u16) << 8;
    token += u[1] as u16;
    Some(token)
}

pub fn token_to_u8(token: u16) -> [u8; 2] {
    let mut bytes = [0; 2];
    bytes[0] = (token >> 8) as u8;
    bytes[1] = token as u8;
    bytes
}

pub fn u8_to_eui(s: &[u8]) -> Option<u64> {
    if s.len() < 8 {
        return None;
    }
    let eui = &s[0..8];
    let mut t = [0; 8];
    t.copy_from_slice(eui);
    Some(u64::from_be_bytes(t))
}

pub fn eui_to_u8(e: u64) -> [u8; 8] {
    e.to_le_bytes()
}
pub fn eui_to_string(e: u64) -> String {
    hex::encode_upper(eui_to_u8(e))
}
pub fn str_to_eui(e: &str) -> Option<u64> {
    let mut buf = [0; 8];
    hex::decode_to_slice(e, &mut buf).ok()?;
    Some(u64::from_le_bytes(buf))
}
