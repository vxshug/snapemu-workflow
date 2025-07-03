use std::ops::Deref;
use std::sync::Arc;

use lorawan::default_crypto::DefaultFactory;

use common_define::db::{Key, LoRaAddr};
use lorawan::parser::{DataHeader, DataPayload, DecryptedDataPayload, EncryptedDataPayload};

use crate::man::data::DataError;
use crate::{DeviceError, DeviceResult};

#[derive(Debug)]
pub(crate) struct LoRaPayload {
    inner: Arc<EncryptedDataPayload<Vec<u8>, DefaultFactory>>,
}

impl Deref for LoRaPayload {
    type Target = EncryptedDataPayload<Vec<u8>, DefaultFactory>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl LoRaPayload {
    pub(crate) fn new(payload: DataPayload<Vec<u8>, DefaultFactory>) -> DeviceResult<Self> {
        match payload {
            DataPayload::Encrypted(en) => Ok(Self { inner: Arc::new(en) }),
            DataPayload::Decrypted(_) => Err(DeviceError::data("PhyPayload Encrypted error")),
        }
    }
}

impl LoRaPayload {
    pub fn dev_addr(&self) -> LoRaAddr {
        let mut buf = [0; 4];
        let dev = self.inner.fhdr().dev_addr();
        buf.copy_from_slice(dev.as_ref());

        buf.into()
    }

    pub(crate) fn decrypt_mic<'a>(
        &self,
        nwk_skey: &'a Key,
        app_skey: &'a Key,
        fcnt: u32,
    ) -> Result<DecryptedDataPayload<Vec<u8>>, DataError> {
        let data = EncryptedDataPayload::new(self.inner.as_data_bytes().to_vec())?;
        let payload = data
            .decrypt_if_mic_ok(nwk_skey, app_skey, fcnt)
            .map_err(|_| DataError::from("decrypt error, nwk_key or app_skey is invalid"))?;
        Ok(payload)
    }
}

impl Clone for LoRaPayload {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}
