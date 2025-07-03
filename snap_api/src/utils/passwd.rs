use crate::utils::base64::Base64;
use crate::utils::Rand;

pub(crate) struct PasswordHash;

impl PasswordHash {
    pub(crate) fn check_password(password: &str, stored_password: &str) -> bool {
        if stored_password.len() < 12 {
            return false;
        }
        let salt = &stored_password[4..12];
        let gen = Self::_gen_password(password, salt);
        gen == stored_password
    }

    pub(crate) fn gen_password(password: &str) -> String {
        let salt = Rand::string(8);
        Self::_gen_password(password, &salt)
    }

    fn _gen_password(password: &str, salt: &str) -> String {
        let hash = Self::hash8192(password, salt);
        let password = Base64::password_encode(hash.as_slice());
        "$P$B".to_owned() + salt + &password
    }

    fn hash8192(password: &str, salt: &str) -> md5::Digest {
        let mut buffer = [0; 100];
        let mut hash = md5::compute(salt.to_owned() + password);
        let len = password.len();
        let count = 1 << 13;
        for _ in 0..count {
            buffer[0..16].copy_from_slice(hash.as_slice());
            buffer[16..16 + len].copy_from_slice(password.as_bytes());
            hash = md5::compute(&buffer[0..16 + len]);
        }
        hash
    }
}

pub(crate) struct Rsa;

impl Rsa {}

#[cfg(test)]
mod tests {
    use crate::utils::PasswordHash;

    #[test]
    fn test_gen_password() {
        assert_ne!(PasswordHash::gen_password("test"), PasswordHash::gen_password("test"));
        println!("{}", PasswordHash::gen_password("test123"));
        assert!(PasswordHash::check_password("123", "$P$B1f90657dW.epYijNBhu1JPuo.mD.c1"))
    }
}
