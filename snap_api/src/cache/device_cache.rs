use crate::service::device::device::DeviceWithAuth;
use common_define::Id;
use derive_new::new;
use redis_macros::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, ToRedisArgs, FromRedisValue, new)]
pub struct DeviceCache {
    pub v: Vec<DeviceWithAuth>,
}

impl DeviceCache {
    fn user_key(id: Id) -> String {
        format!("cache:device:{}", id)
    }

    pub async fn delete_by_user_id<C: redis::aio::ConnectionLike>(
        user_id: Id,
        r: &mut C,
    ) -> redis::RedisResult<()> {
        let key = Self::user_key(user_id);
        redis::Cmd::del(key).query_async(r).await
    }
    pub async fn save_by_user_id<C: redis::aio::ConnectionLike>(
        &self,
        user_id: Id,
        r: &mut C,
    ) -> redis::RedisResult<()> {
        let key = Self::user_key(user_id);
        redis::Cmd::set(key, self).query_async(r).await
    }
    pub async fn load_by_user_id<C: redis::aio::ConnectionLike>(
        user_id: Id,
        r: &mut C,
    ) -> redis::RedisResult<Option<Self>> {
        let key = Self::user_key(user_id);
        redis::Cmd::get(key).query_async(r).await
    }
}
