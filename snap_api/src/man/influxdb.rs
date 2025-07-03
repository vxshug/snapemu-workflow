use derive_new::new;
use influxdb2::models::Query;
use influxdb2::RequestError;
use crate::service::data::DeviceData;

#[derive(Debug, thiserror::Error)]
pub enum InfluxError {
    #[error("InfluxDB error: {0}")]
    RequestError(#[from] RequestError)
}

pub struct InfluxQueryBuilder<'a> {
    fns: Vec<&'a str>,
    bucket: Option<&'a str>,
}

impl<'a> InfluxQueryBuilder<'a> {
    pub fn new() -> Self {
        Self {
            fns: Vec::new(),
            bucket: None,
        }
    }

    pub fn new_with_fns(fns: Vec<&'a str>) -> Self {
        Self {
            fns,
            bucket: None,
        }
    }

    pub fn bucket(mut self, bucket: &'a str) -> Self {
        self.bucket = Some(bucket);
        self
    }

    pub fn add_fn(mut self, fn_name: &'a str) -> Self {
        self.fns.push(fn_name);
        self
    }

    pub fn add_fn_from_slice<>(mut self, fn_name: &[&'a str]) -> Self {
        self.fns.extend_from_slice(fn_name);
        self
    }

    fn build(self, default_bucket: &str) -> String {
        let bucket = self.bucket.unwrap_or(default_bucket);
        let fns = self.fns.join("\n");
        format!("from(bucket: \"{}\")\n{}", bucket, fns)
    }
}

#[derive(new, Clone)]
pub struct InfluxDbClient {
    bucket: String,
    client: influxdb2::Client
}

impl InfluxDbClient {
    pub async fn query(&self, query_builder: InfluxQueryBuilder<'_>) -> Result<Vec<DeviceData>, InfluxError> {
        let query = query_builder.build(self.bucket.as_ref());
        Ok(self.client.query(Some(Query::new(query))).await?)
    }
}
