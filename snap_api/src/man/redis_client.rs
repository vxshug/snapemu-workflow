use crate::error::{ApiError, ApiResult};
use crate::load::load_config;
use std::sync::Mutex;
use tokio_stream::Stream;
use tracing::log::warn;

static LOCAL_CLIENT: Mutex<Option<RedisClient>> = Mutex::new(None);

#[derive(Clone)]
pub struct RedisClient {
    client: redis::Client,
}

impl RedisClient {
    pub fn init() -> Self {
        let config = load_config();
        let host = config.redis.host.clone();
        let port = config.redis.port;
        let db = config.redis.db;
        let username = config.redis.username.clone().unwrap_or_default();
        let password = config.redis.password.clone().unwrap_or_default();
        let url = format!("redis://{}:{}@{}:{}/{}", username, password, host, port, db);
        let client = Self {
            client: redis::Client::open(url.as_str()).expect("Failed to open redis client"),
        };
        let _ = LOCAL_CLIENT.lock().unwrap().replace(client.clone());
        client
    }

    pub fn get_client() -> Self {
        {
            if let Some(client) = LOCAL_CLIENT.lock().unwrap().as_ref().map(|client| client.clone())
            {
                return client;
            }
        }
        Self::init()
    }

    pub fn reconnect() -> Self {
        warn!("redis reconnect");
        Self::init()
    }

    pub async fn get_pubsub(&self) -> ApiResult<redis::aio::PubSub> {
        Ok(self.client.get_async_pubsub().await?)
    }

    pub async fn get_multiplexed_conn(&self) -> ApiResult<redis::aio::MultiplexedConnection> {
        Ok(self.client.get_multiplexed_tokio_connection().await?)
    }
    pub async fn get(&self) -> ApiResult<redis::aio::MultiplexedConnection> {
        Ok(self.client.get_multiplexed_tokio_connection().await?)
    }
}

pub struct RedisRecv {
    conn: redis::aio::PubSub,
}

impl RedisRecv {
    pub fn new(conn: redis::aio::PubSub) -> Self {
        Self { conn }
    }

    pub async fn reconnect(&mut self) -> Option<()> {
        let s = RedisClient::get_client();
        self.conn = s.get_pubsub().await.ok()?;
        Some(())
    }
    pub async fn subscribe(&mut self, topic: &str) -> ApiResult {
        match self.conn.subscribe(topic).await {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.is_io_error() {
                    self.reconnect()
                        .await
                        .ok_or(ApiError::Server { case: "redis", msg: "redis reconnect".into() })?;
                    self.conn.subscribe(topic).await?;
                    return Ok(());
                }
                Err(ApiError::Server { case: "redis", msg: "redis reconnect error".into() })
            }
        }
    }

    pub async fn unsubscribe(&mut self, topic: &str) -> ApiResult {
        match self.conn.unsubscribe(topic).await {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.is_io_error() {
                    self.reconnect().await.ok_or(ApiError::Server {
                        case: "redis",
                        msg: "redis unsubscribe".into(),
                    })?;
                    self.conn.unsubscribe(topic).await?;
                    return Ok(());
                }
                Err(ApiError::Server { case: "redis", msg: "redis unsubscribe error".into() })
            }
        }
    }

    pub fn message(&mut self) -> impl Stream<Item = redis::Msg> + '_ {
        self.conn.on_message()
    }

    pub fn into_on_message(self) -> impl Stream<Item = redis::Msg> {
        self.conn.into_on_message()
    }
}
