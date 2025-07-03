use std::collections::HashMap;

pub struct SnapConfigBuilder {
    builder: config::ConfigBuilder<config::builder::DefaultState>,
    env_prefix: String,
    env_separator: Option<String>,
}

impl SnapConfigBuilder {
    pub fn env_separator(mut self, env_separator: &str) -> Self {
        self.env_separator = Some(env_separator.to_owned());
        self
    }

    pub fn env_prefix(mut self, env_prefix: &str) -> Self {
        self.env_prefix = env_prefix.to_owned();
        self
    }

    pub fn add_file(mut self, file: &str) -> Self {
        self.builder = self.builder.add_source(config::File::with_name(file));
        self
    }

    pub fn set_default<V: Into<config::Value>>(
        mut self,
        key: &str,
        value: V,
    ) -> anyhow::Result<Self> {
        self.builder = self.builder.set_default(key, value)?;
        Ok(self)
    }
    pub fn build(self) -> anyhow::Result<SnapConfig> {
        let mut env = config::Environment::with_prefix(&self.env_prefix);
        if let Some(env_separator) = self.env_separator {
            env = env.separator(&env_separator);
        }
        let config = self.builder.add_source(env).build()?;
        Ok(SnapConfig { config })
    }
}

#[derive(Clone)]
pub struct SnapConfig {
    config: config::Config,
}

impl SnapConfig {
    pub fn builder() -> SnapConfigBuilder {
        SnapConfigBuilder {
            builder: config::builder::ConfigBuilder::default(),
            env_prefix: "SNAP_".to_string(),
            env_separator: None,
        }
    }

    pub fn get_table(&self, key: &str) -> anyhow::Result<HashMap<String, crate::Value>> {
        self.config.get_table(key).map_err(Into::into)
    }

    pub fn get_string(&self, key: &str) -> anyhow::Result<String> {
        self.config.get_string(key).map_err(Into::into)
    }
    pub fn get_i64(&self, key: &str) -> anyhow::Result<i64> {
        self.config.get_int(key).map_err(Into::into)
    }
    pub fn get_f64(&self, key: &str) -> anyhow::Result<f64> {
        self.config.get_float(key).map_err(Into::into)
    }
    pub fn get_bool(&self, key: &str) -> anyhow::Result<bool> {
        self.config.get_bool(key).map_err(Into::into)
    }
    pub fn get_array(&self, key: &str) -> anyhow::Result<Vec<crate::Value>> {
        self.config.get_array(key).map_err(Into::into)
    }

    pub fn into_local_config<T: serde::de::DeserializeOwned>(self) -> anyhow::Result<T> {
        Ok(self.config.try_deserialize()?)
    }
}
