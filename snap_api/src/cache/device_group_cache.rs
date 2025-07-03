use common_define::db::DeviceGroupModel;
use common_define::Id;
use derive_new::new;
use redis_macros::{FromRedisValue, ToRedisArgs};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, ToRedisArgs, FromRedisValue, new)]
pub struct DeviceGroupCache {
    pub v: Vec<DeviceGroupCacheItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, new)]
pub struct DeviceGroupCacheItem {
    #[serde(flatten)]
    pub group: DeviceGroupModel,
    pub count: i64,
}

impl DeviceGroupCache {
    fn user_group_key(id: Id) -> String {
        format!("cache:group:{}", id)
    }

    pub async fn delete_by_user_id<C: redis::aio::ConnectionLike>(
        user_id: Id,
        r: &mut C,
    ) -> redis::RedisResult<()> {
        let key = Self::user_group_key(user_id);
        redis::Cmd::del(key).query_async(r).await
    }

    pub async fn load_default_by_user_id<C: redis::aio::ConnectionLike>(
        user_id: Id,
        r: &mut C,
    ) -> redis::RedisResult<Option<DeviceGroupCacheItem>> {
        let caches = Self::load_by_user_id(user_id, r).await?;
        match caches {
            Some(caches) => {
                for cache in caches.v {
                    if cache.group.default_group {
                        return Ok(Some(cache));
                    }
                }
            }
            None => return Ok(None),
        }
        Ok(None)
    }
    pub async fn save_by_user_id<C: redis::aio::ConnectionLike>(
        groups: Self,
        user_id: Id,
        r: &mut C,
    ) -> redis::RedisResult<()> {
        let key = Self::user_group_key(user_id);
        redis::Cmd::set(key, groups).query_async(r).await
    }
    pub async fn load_by_user_id<C: redis::aio::ConnectionLike>(
        user_id: Id,
        r: &mut C,
    ) -> redis::RedisResult<Option<Self>> {
        let key = Self::user_group_key(user_id);
        redis::Cmd::get(key).query_async(r).await
    }
}
