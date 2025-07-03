use std::str::FromStr;

pub struct Checker;

const HEX: &[u8; 22] = b"0123456789abcdefABCDEF";
const USERNAME: &[u8; 63] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-";

fn slice_include(c: u8, table: &[u8]) -> bool {
    for h in table {
        if *h == c {
            return true;
        }
    }
    false
}
impl Checker {
    pub fn email(email: &str) -> bool {
        lettre::Address::from_str(email).is_ok()
    }

    pub fn username(username: &str) -> bool {
        for char in username.as_bytes() {
            if !slice_include(*char, USERNAME) {
                return false;
            }
        }
        true
    }
    pub fn hex(hex: &str) -> bool {
        for char in hex.as_bytes() {
            if !slice_include(*char, HEX) {
                return false;
            }
        }
        true
    }
    pub fn check_eui(eui: &str, scan_eui: &str) -> bool {
        if eui.len() == 16 && scan_eui.len() == 32 {
            &scan_eui.as_bytes()[16..] == eui.as_bytes()
        } else {
            false
        }
    }
}
