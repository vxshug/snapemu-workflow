use common_define::db::{Eui, Key};
use lorawan::default_crypto::DefaultFactory;
use lorawan::parser::JoinRequestPayload;

#[derive(Debug)]
pub(crate) struct RequestJoin(JoinRequestPayload<Vec<u8>, DefaultFactory>);

impl RequestJoin {
    pub(crate) fn new(re: JoinRequestPayload<Vec<u8>, DefaultFactory>) -> Self {
        Self(re)
    }

    pub(crate) fn app_eui(&self) -> Eui {
        let mut buf = [0; 8];
        buf.copy_from_slice(self.0.app_eui().as_ref());
        buf.into()
    }

    pub(crate) fn dev_eui(&self) -> Eui {
        let mut buf = [0; 8];
        buf.copy_from_slice(self.0.dev_eui().as_ref());
        buf.into()
    }

    pub(crate) fn dev_nonce(&self) -> u16 {
        let nonce = self.0.dev_nonce();
        let u = nonce.as_ref();
        let mut dev_nonce: u16 = u[1] as u16;
        dev_nonce += (u[0] as u16) << 8;
        dev_nonce
    }

    pub(crate) fn validate(&self, key: &Key) -> bool {
        self.0.validate_mic(&key.0)
    }
}
