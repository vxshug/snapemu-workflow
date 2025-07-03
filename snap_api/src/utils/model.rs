use std::collections::HashMap;

#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, Debug)]
pub(crate) enum ValueType {
    Unknown,
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

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub(crate) struct ModelItem {
    name: HashMap<String, String>,
    pub(crate) unit: String,
    #[serde(rename = "type")]
    pub(crate) value_type: ValueType,
}

impl ModelItem {
    pub(crate) fn name(&self, lang: &str) -> String {
        self.name.get(lang).or(self.name.get("en")).cloned().unwrap_or(String::from("Unkown"))
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(transparent)]
pub(crate) struct Model(HashMap<u32, HashMap<u32, ModelItem>>);

impl Model {
    pub(crate) fn item(&self, sensor: u32, pk: u32) -> Option<ModelItem> {
        self.0.get(&sensor)?.get(&pk).cloned()
    }
}
