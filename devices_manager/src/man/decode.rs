use crate::decode::{DecodeData, JsDecodeError, JsManager, RawData};
use crate::man::Id;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};



#[derive(Clone)]
pub struct DecodeManager {
    rt: JsManager,
}

impl DecodeManager {
    pub fn new(rt: JsManager) -> Self {
        Self { rt }
    }

    pub fn decode_with_code(
        &self,
        code: &str,
        data: RawData,
    ) -> Result<DecodeData, JsDecodeError> {
        let data = self.rt.eval(code, data)?;
        Ok(data)
    }
}
