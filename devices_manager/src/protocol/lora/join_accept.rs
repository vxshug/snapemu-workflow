use common_define::db::Key;
use generic_array::typenum::U16;
use generic_array::GenericArray;
use lorawan::creator::JoinAcceptCreator;
use lorawan::default_crypto::DefaultFactory;
use lorawan::keys::{self, CryptoFactory, Encrypter};
use lorawan::maccommands::Frequency;

use crate::man::data::DataError;

pub(crate) struct AcceptJoin {
    inner: JoinAcceptCreator<[u8; 33], DefaultFactory>,
    len: usize,
}

impl AcceptJoin {
    pub(crate) fn new() -> Self {
        let v: [u8; 33] = [0; 33];
        let inner = JoinAcceptCreator::with_options(v, DefaultFactory).unwrap();
        Self { inner, len: 17 }
    }
    pub(crate) fn set_app_nonce(&mut self, app_nonce: u32) -> &mut Self {
        let mut i_app_nonce = [0; 3];
        i_app_nonce[0] = (app_nonce >> 16) as u8;
        i_app_nonce[1] = (app_nonce >> 8) as u8;
        i_app_nonce[2] = app_nonce as u8;
        self.inner.set_app_nonce(&i_app_nonce);
        self
    }
    pub(crate) fn set_net_id(&mut self, net_id: u32) -> &mut Self {
        let mut i_net_id = [0; 3];
        i_net_id[0] = (net_id >> 16) as u8;
        i_net_id[1] = (net_id >> 8) as u8;
        i_net_id[2] = net_id as u8;
        self.inner.set_net_id(&i_net_id);
        self
    }
    pub(crate) fn set_dev_addr(&mut self, dev_addr: u32) -> &mut Self {
        let dev_addr = dev_addr.to_le_bytes();
        self.inner.set_dev_addr(&dev_addr);
        self
    }
    pub(crate) fn set_dl_settings(&mut self, dl_settings: u8) -> &mut Self {
        self.inner.set_dl_settings(dl_settings);
        self
    }
    pub(crate) fn set_rx_delay(&mut self, rx_delay: u8) -> &mut Self {
        self.inner.set_rx_delay(rx_delay);
        self
    }
    pub(crate) fn set_c_f_list(&mut self, list: Vec<[u8; 3]>) -> Result<(), DataError> {
        let mut freq = Vec::new();
        for l in &list {
            freq.push(Frequency::new(l).ok_or(String::from("CFList set fault"))?);
            self.len += 3;
        }
        self.inner.set_c_f_list(freq)?;
        Ok(())
    }
    pub(crate) fn build(&mut self, key: &keys::AES128) -> Result<&[u8], DataError> {
        let x = self.inner.build(key)?;
        Ok(&x[0..self.len])
    }
}
pub(crate) struct NodeKeys {
    pub(crate) nwk_skey: Key,
    pub(crate) app_skey: Key,
}

impl NodeKeys {
    pub(crate) fn new(app_key: &Key, join_nonce: u32, net_id: u32, dev_nonce: u16) -> Self {
        let mut nwk_skey = [0; 16];
        let mut app_skey = [0; 16];
        let k1: &mut GenericArray<u8, U16> = GenericArray::from_mut_slice(&mut nwk_skey);
        let k2: &mut GenericArray<u8, U16> = GenericArray::from_mut_slice(&mut app_skey);
        k1[0] = 1;
        k2[0] = 2;
        Self::fill_args(k1.as_mut_slice(), join_nonce, net_id, dev_nonce);
        Self::fill_args(k2.as_mut_slice(), join_nonce, net_id, dev_nonce);
        let enc = DefaultFactory.new_enc(app_key);
        enc.encrypt_block(k1);
        enc.encrypt_block(k2);

        Self { nwk_skey: Key::new(nwk_skey), app_skey: Key::new(app_skey) }
    }

    fn fill_args(s: &mut [u8], join_nonce: u32, net_id: u32, dev_nonce: u16) {
        let join_nonce = join_nonce.to_be_bytes();
        s[1] = join_nonce[1];
        s[2] = join_nonce[2];
        s[3] = join_nonce[3];

        let net_id = net_id.to_be_bytes();
        s[4] = net_id[1];
        s[5] = net_id[2];
        s[6] = net_id[3];

        let dev_nonce = dev_nonce.to_be_bytes();

        s[7] = dev_nonce[0];
        s[8] = dev_nonce[1];
    }
}
#[cfg(test)]
mod tests {}
