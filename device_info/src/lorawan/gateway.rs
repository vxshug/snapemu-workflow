use crate::MyOption;
use common_define::db::Eui;
use common_define::time::Timestamp;
use common_define::Id;
use derive_new::new;
use hash_name::{HashNames, RedisOps};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tracing::instrument;

#[derive(Debug, Clone, Serialize, Deserialize, RedisOps, new, HashNames)]
pub struct GatewayInfo {
    pub device: Id,
    pub tmst: u32,
    pub version: u8,
    pub time: Timestamp,
    pub a: Option<Timestamp>,
    pub down: Option<String>,
}

impl GatewayInfo {
    pub fn eui_key(eui: Eui) -> String {
        format!("info:gateway:{}", eui)
    }

    #[instrument(skip(conn))]
    pub async fn check_eui<C: redis::aio::ConnectionLike>(
        dev_eui: Eui,
        conn: &mut C,
    ) -> redis::RedisResult<bool> {
        let k1 = Self::eui_key(dev_eui);
        redis::Cmd::exists(&k1).query_async(conn).await
    }
    pub async fn register<C: redis::aio::ConnectionLike>(
        &self,
        eui: Eui,
        con: &mut C,
    ) -> redis::RedisResult<()> {
        let k = format!("info:gateway:{}", eui);
        redis::cmd("HSET").arg(&k).arg(self).query_async(con).await
    }

    pub async fn unregister<C: redis::aio::ConnectionLike>(
        eui: Eui,
        con: &mut C,
    ) -> redis::RedisResult<()> {
        let k = format!("info:gateway:{}", eui);
        redis::Cmd::del(&k).query_async(con).await
    }
    pub async fn update_active_time<C: redis::aio::ConnectionLike>(
        eui: Eui,
        con: &mut C,
    ) -> redis::RedisResult<Option<Timestamp>> {
        let k = Self::eui_key(eui);
        redis::Cmd::hset(&k, Self::a(), Timestamp::now()).query_async(con).await
    }

    pub async fn update_download<C: redis::aio::ConnectionLike>(
        eui: Eui,
        addr: SocketAddr,
        con: &mut C,
    ) -> redis::RedisResult<Option<Timestamp>> {
        let k = Self::eui_key(eui);
        redis::Cmd::hset(&k, Self::down(), addr.to_string()).query_async(con).await
    }
    pub async fn load_active_time<C: redis::aio::ConnectionLike>(
        eui: Eui,
        con: &mut C,
    ) -> redis::RedisResult<Option<Timestamp>> {
        let k = Self::eui_key(eui);
        redis::Cmd::hget(&k, Self::a()).query_async(con).await
    }
    pub async fn load<C: redis::aio::ConnectionLike>(
        eui: Eui,
        con: &mut C,
    ) -> redis::RedisResult<Option<Self>> {
        let k = format!("info:gateway:{}", eui);
        redis::Cmd::hgetall(&k).query_async::<MyOption<Self>>(con).await.map(Into::into)
    }
}
