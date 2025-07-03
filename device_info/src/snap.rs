use crate::MyOption;
use common_define::db::{Eui, Key};
use common_define::time::Timestamp;
use common_define::Id;
use derive_new::new;
use hash_name::{HashNames, RedisOps};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Serialize, Deserialize, RedisOps, HashNames, new)]
pub struct SnapDeviceInfo {
    pub id: Id,
    pub key: Key,
    pub active: Option<Timestamp>,
    pub up_count: u32,
    pub down: Option<String>,
    pub script: Option<Id>,
    pub freq: Option<f32>,
}

impl SnapDeviceInfo {
    pub fn eui_key(eui: Eui) -> String {
        format!("info:snap:{}", eui)
    }

    #[instrument(skip(conn))]
    pub async fn check_eui<C: redis::aio::ConnectionLike>(
        dev_eui: Eui,
        conn: &mut C,
    ) -> redis::RedisResult<bool> {
        let k1 = Self::eui_key(dev_eui);
        redis::Cmd::exists(&k1).query_async(conn).await
    }
    #[instrument(skip(conn, v))]
    pub async fn update_by_eui<C: redis::aio::ConnectionLike, V: redis::ToRedisArgs>(
        eui: Eui,
        key: &str,
        v: V,
        conn: &mut C,
    ) -> redis::RedisResult<()> {
        let k = Self::eui_key(eui);
        if redis::Cmd::exists(&k).query_async(conn).await? {
            () = redis::cmd("HSET").arg(&k).arg(key).arg(v).query_async(conn).await?;
        }
        Ok(())
    }
    pub async fn register<C: redis::aio::ConnectionLike>(
        &self,
        eui: Eui,
        con: &mut C,
    ) -> redis::RedisResult<()> {
        let k = format!("info:snap:{}", eui);
        redis::cmd("HSET").arg(k).arg(self).query_async(con).await
    }

    pub async fn unregister<C: redis::aio::ConnectionLike>(
        eui: Eui,
        con: &mut C,
    ) -> redis::RedisResult<()> {
        let k = format!("info:snap:{}", eui);
        redis::Cmd::del(&k).query_async(con).await
    }

    pub async fn update_active_time<C: redis::aio::ConnectionLike>(
        eui: Eui,
        timestamp: Timestamp,
        con: &mut C,
    ) -> redis::RedisResult<Option<Timestamp>> {
        let k = format!("info:snap:{}", eui);
        redis::Cmd::hset(&k, Self::active(), timestamp).query_async(con).await
    }
    pub async fn load_active_time<C: redis::aio::ConnectionLike>(
        eui: Eui,
        con: &mut C,
    ) -> redis::RedisResult<Option<Timestamp>> {
        let k = format!("info:snap:{}", eui);
        redis::Cmd::hget(&k, Self::active()).query_async(con).await
    }

    pub async fn load_down<C: redis::aio::ConnectionLike>(
        eui: Eui,
        con: &mut C,
    ) -> redis::RedisResult<Option<String>> {
        let k = format!("info:snap:{}", eui);
        redis::Cmd::hget(&k, Self::down()).query_async(con).await
    }

    pub async fn update_down<C: redis::aio::ConnectionLike>(
        eui: Eui,
        down: String,
        con: &mut C,
    ) -> redis::RedisResult<Option<String>> {
        let k = format!("info:snap:{}", eui);
        redis::Cmd::hset(&k, Self::down(), down).query_async(con).await
    }

    pub async fn load_freq<C: redis::aio::ConnectionLike>(
        eui: Eui,
        con: &mut C,
    ) -> redis::RedisResult<Option<String>> {
        let k = format!("info:snap:{}", eui);
        redis::Cmd::hget(&k, Self::freq()).query_async(con).await
    }

    pub async fn update_freq<C: redis::aio::ConnectionLike>(
        eui: Eui,
        freq: f32,
        con: &mut C,
    ) -> redis::RedisResult<Option<String>> {
        let k = format!("info:snap:{}", eui);
        redis::Cmd::hset(&k, Self::freq(), freq).query_async(con).await
    }
    pub async fn load<C: redis::aio::ConnectionLike>(
        eui: Eui,
        con: &mut C,
    ) -> redis::RedisResult<Option<Self>> {
        let k = format!("info:snap:{}", eui);
        redis::Cmd::hgetall(&k).query_async::<MyOption<Self>>(con).await.map(Into::into)
    }
}
