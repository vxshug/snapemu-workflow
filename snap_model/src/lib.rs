use common_define::lora::ValueType;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;

#[derive(Deserialize, Debug)]
pub struct ModelRoot {
    #[serde(rename = "defaultModel")]
    default_model: DefaultModel,
    models: Vec<Model>,
}

#[derive(Deserialize, Debug)]
pub struct DefaultModel {
    unit: String,
    name: ModelPackageName,
}

#[derive(Deserialize, Debug)]
pub struct Model {
    #[serde(rename = "sensorID")]
    sensor_id: u32,
    #[serde(rename = "sensorName")]
    sensor_name: String,
    packages: Vec<ModelPackage>,
}

#[derive(Deserialize, Debug)]
pub struct ModelPackage {
    id: u8,
    unit: String,
    #[serde(rename = "type")]
    v_type: ValueType,
    name: ModelPackageName,
}

#[derive(Deserialize, Debug)]
pub struct ModelPackageName {
    zh: String,
    en: String,
}

#[derive(Debug)]
pub struct ModelEntity {
    pub unit: &'static str,
    pub v_type: Option<ValueType>,
    pub name: &'static str,
}

#[derive(Debug, Clone)]
pub struct DefaultEntry {
    unit: &'static str,
    name: ModelEntityName,
}
#[derive(Debug, Clone)]
pub struct ModelMapEntry {
    unit: &'static str,
    v_type: ValueType,
    name: ModelEntityName,
}
#[derive(Debug, Clone)]
pub struct ModelEntityName {
    zh: &'static str,
    en: &'static str,
}
#[derive(Debug, Clone)]
pub struct ModelMap {
    map: BTreeMap<u32, ModelMapEntry>,
    default: DefaultEntry,
}

impl Default for ModelMap {
    fn default() -> Self {
        Self {
            map: Default::default(),
            default: DefaultEntry {
                unit: "unknown",
                name: ModelEntityName { zh: "unknown", en: "unknown" },
            },
        }
    }
}

impl ModelMap {
    fn new_with_root(root: ModelRoot) -> Self {
        let mut map = BTreeMap::new();
        for model in root.models {
            for pk in model.packages {
                assert!(pk.id < 0x10);
                assert!(model.sensor_id < 0x1_00_00);
                let id = (model.sensor_id << 4) | pk.id as u32;
                map.insert(
                    id,
                    ModelMapEntry {
                        unit: pk.unit.leak(),
                        v_type: pk.v_type,
                        name: ModelEntityName { zh: pk.name.zh.leak(), en: pk.name.en.leak() },
                    },
                );
            }
        }

        Self {
            map,
            default: DefaultEntry {
                unit: root.default_model.unit.leak(),
                name: ModelEntityName {
                    zh: root.default_model.name.zh.leak(),
                    en: root.default_model.name.en.leak(),
                },
            },
        }
    }

    pub fn get_entry(&self, data_id: u32, lang: &str) -> ModelEntity {
        match self.map.get(&data_id) {
            None => self.get_default_entry(lang),
            Some(s) => ModelEntity {
                unit: s.unit,
                v_type: Some(s.v_type),
                name: if lang == "zh" { s.name.zh } else { s.name.en },
            },
        }
    }

    pub fn get_default_entry(&self, lang: &str) -> ModelEntity {
        ModelEntity {
            unit: self.default.unit,
            v_type: None,
            name: if lang == "zh" { self.default.name.zh } else { self.default.name.en },
        }
    }
}

pub fn load_model_file(path: Option<&str>) -> ModelMap {
    match path {
        Some(path) => {
            let config = config::Config::builder()
                .add_source(config::File::with_name(path))
                .build()
                .unwrap();
            let root: ModelRoot = config.try_deserialize().unwrap();
            ModelMap::new_with_root(root)
        }
        None => ModelMap::default(),
    }
}

impl ModelSource for ModelMap {
    fn get_data_name(&self, data_id: u32) -> DataMap {
        if let Some(m) = self.map.get(&data_id) {
            return DataMap {
                unit: m.unit,
                v_type: m.v_type,
                name: DataName {
                    zh: m.name.zh,
                    en: m.name.en,
                }
            }
        }
        DataMap {
            unit: self.default.unit,
            v_type: ValueType::F32,
            name: DataName {
                zh: self.default.name.zh,
                en: self.default.name.en,
            }
        }
    }
}

impl<T: ModelSource> ModelSource for &T {
    fn get_data_name(&self, data_id: u32) -> DataMap {
        (*self).get_data_name(data_id)
    }
}

#[derive(Debug, Clone)]
pub struct DataMap<'a> {
    pub unit: &'a str,
    pub v_type: ValueType,
    pub name: DataName<'a>,
}
#[derive(Debug, Clone)]
pub struct DataName<'a> {
    zh: &'a str,
    en: &'a str,
}

impl<'a> From<&'a str> for DataName<'a> {
    fn from(s: &'a str) -> DataName<'a> {
        if let Some((zh, en)) = s.split_once(',') {
            return Self {
                zh,
                en
            }
        }
        Self {
            zh: s,
            en: s,
        }
    }
}

impl DataName<'_> {
    pub fn format_tag(&self) -> String {
        format!("{},{}", self.zh, self.en)
    }
}

pub trait ModelSource {
    fn get_data_name(&self, data_id: u32) -> DataMap;
}