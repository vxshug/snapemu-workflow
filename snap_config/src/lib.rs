mod load;

pub use config::Value;
pub use load::SnapConfig;
pub use load::SnapConfigBuilder;

pub use database::DatabaseConfig;
pub use device_topic::DeviceTopicConfig;
pub use log_level::init_logging;
pub use log_level::LogLevelConfig;
pub use redis::RedisConfig;
pub use tsdb::TsdbConfig;

mod redis {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct RedisConfig {
        #[serde(default = "_default_redis_host")]
        pub host: String,
        #[serde(default = "_default_redis_port")]
        pub port: i64,
        #[serde(default = "_default_redis_db")]
        pub db: u8,
        #[serde(default)]
        pub username: Option<String>,
        #[serde(default)]
        pub password: Option<String>,
        #[serde(default = "_default_redis_pool")]
        pub pool: usize,
    }

    fn _default_redis_host() -> String {
        "localhost".to_string()
    }
    fn _default_redis_port() -> i64 {
        6379
    }
    fn _default_redis_db() -> u8 {
        0
    }
    fn _default_redis_pool() -> usize {
        100
    }

    impl Default for RedisConfig {
        fn default() -> Self {
            Self {
                host: _default_redis_host(),
                port: _default_redis_port(),
                db: _default_redis_db(),
                username: None,
                password: None,
                pool: _default_redis_pool(),
            }
        }
    }
}

mod database {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct DatabaseConfig {
        #[serde(default = "_default_database_host")]
        pub host: String,
        #[serde(default = "_default_database_port")]
        pub port: i64,
        #[serde(default = "_default_database_db")]
        pub db: String,
        #[serde(default = "_default_database_connection_count")]
        pub connection_count: u32,
        #[serde(default = "_default_database_username")]
        pub username: String,
        #[serde(default = "_default_database_password")]
        pub password: String,
        #[serde(default)]
        pub sqlx_logging: bool,
    }

    fn _default_database_host() -> String {
        "localhost".to_string()
    }
    fn _default_database_port() -> i64 {
        5432
    }
    fn _default_database_db() -> String {
        "snapemu".to_string()
    }
    fn _default_database_connection_count() -> u32 {
        10
    }
    fn _default_database_username() -> String {
        "postgres".to_string()
    }
    fn _default_database_password() -> String {
        "postgres".to_string()
    }

    impl Default for DatabaseConfig {
        fn default() -> Self {
            Self {
                host: _default_database_host(),
                port: _default_database_port(),
                db: _default_database_db(),
                connection_count: _default_database_connection_count(),
                username: _default_database_username(),
                password: _default_database_password(),
                sqlx_logging: false,
            }
        }
    }
}

mod device_topic {
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    pub struct DeviceTopicConfig {
        #[serde(default = "_default_data_topic")]
        pub data: String,
        #[serde(default = "_default_event_topic")]
        pub event: String,
        #[serde(default = "_default_down_topic")]
        pub down: String,
    }
    fn _default_data_topic() -> String {
        "LoRa-Push-Data".to_string()
    }
    fn _default_event_topic() -> String {
        "LoRa-Event".to_string()
    }
    fn _default_down_topic() -> String {
        "LoRa-Down-Data".to_string()
    }

    impl Default for DeviceTopicConfig {
        fn default() -> Self {
            Self {
                data: _default_data_topic(),
                event: _default_event_topic(),
                down: _default_down_topic(),
            }
        }
    }
}

mod log_level {
    use serde::Deserialize;
    use tracing::level_filters::LevelFilter;

    #[derive(Deserialize, Debug, Copy, Clone)]
    pub enum LogLevelConfig {
        TRACE,
        DEBUG,
        INFO,
        WARN,
        ERROR,
    }

    impl From<LogLevelConfig> for LevelFilter {
        fn from(value: LogLevelConfig) -> Self {
            match value {
                LogLevelConfig::TRACE => Self::TRACE,
                LogLevelConfig::DEBUG => Self::DEBUG,
                LogLevelConfig::INFO => Self::INFO,
                LogLevelConfig::WARN => Self::WARN,
                LogLevelConfig::ERROR => Self::ERROR,
            }
        }
    }

    impl Default for LogLevelConfig {
        fn default() -> Self {
            Self::INFO
        }
    }

    pub fn init_logging(level: LogLevelConfig) {
        let log_level: LevelFilter = level.into();
        tracing_subscriber::fmt().with_max_level(log_level).init();
    }
}

mod tsdb {
    use serde::Deserialize;

    #[derive(Deserialize, Debug, Clone)]
    pub struct TsdbConfig {
        #[serde(default = "_default_host")]
        pub host: String,
        #[serde(default = "_default_org")]
        pub org: String,
        #[serde(default = "_default_token")]
        pub token: String,
        #[serde(default = "_default_bucket")]
        pub bucket: String,
    }

    fn _default_host() -> String {
        "http://loclhost:8086".to_string()
    }
    fn _default_org() -> String {
        "".to_string()
    }
    fn _default_token() -> String {
        "".to_string()
    }
    fn _default_bucket() -> String {
        "".to_string()
    }
    impl Default for TsdbConfig {
        fn default() -> Self {
            Self {
                host: _default_host(),
                org: _default_org(),
                token: _default_token(),
                bucket: _default_bucket(),
            }
        }
    }
}
