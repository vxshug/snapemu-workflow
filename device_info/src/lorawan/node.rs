use crate::MyOption;
use common_define::db::{DeviceLoraNodeModel, DevicesModel, Eui, Key, LoRaAddr};
use common_define::lora::{LoRaJoinType, LoRaRegion};
use common_define::product::ProductType;
use common_define::time::Timestamp;
use common_define::Id;
use hash_name::{HashNames, RedisOps};
use serde::Serialize;
use tracing::{instrument, warn};

#[derive(Debug, Serialize, HashNames, RedisOps)]
pub struct NodeInfo {
    pub user_id: Option<Id>,
    pub device_id: Id,
    pub region: LoRaRegion,
    pub join_type: LoRaJoinType,
    pub app_eui: Eui,
    pub dev_eui: Eui,
    pub app_key: Key,
    pub dev_addr: LoRaAddr,
    pub nwk_skey: Key,
    pub app_skey: Key,
    pub class_b: bool,
    pub class_c: bool,
    pub adr: bool,
    pub rx1_delay: i16,
    pub des_rx1_delay: i16,
    pub rx1_dro: i16,
    pub des_rx1_dro: i16,
    pub rx2_dr: i16,
    pub des_rx2_dr: i16,
    pub rx2_freq: i32,
    pub des_rx2_freq: i32,
    pub d_retry: i16,
    pub c_retry: i16,
    pub product_type: ProductType,
    pub dutycyle: i32,
    pub up_confirm: bool,
    pub up_dr: i16,
    pub up_count: u32,
    pub down_count: u32,
    pub power: i16,
    pub battery: Option<i16>,
    pub charge: bool,
    pub time_zone: i32,
    pub firmware: i32,
    pub dev_non: i32,
    pub app_non: i32,
    pub net_id: i32,
    pub enable: bool,
    pub period: i32,
    pub script: Option<Id>,
    pub active_time: Option<Timestamp>,

    pub gateway: Option<Eui>,
}

impl NodeInfo {
    fn eui_key(dev_eui: Eui) -> String {
        format!("info:eui:node:{}", dev_eui)
    }
    pub fn addr_key(addr: LoRaAddr) -> String {
        format!("info:node:{}", addr)
    }

    #[instrument(skip(conn, v))]
    pub async fn update_by_addr<C: redis::aio::ConnectionLike, V: redis::ToRedisArgs>(
        addr: LoRaAddr,
        key: &str,
        v: V,
        conn: &mut C,
    ) -> redis::RedisResult<()> {
        let addr_key = Self::addr_key(addr);
        redis::cmd("HSET").arg(&addr_key).arg(key).arg(v).query_async(conn).await
    }

    #[instrument(skip(conn))]
    pub async fn reset_by_eui<C: redis::aio::ConnectionLike>(
        eui: Eui,
        key: &str,
        conn: &mut C,
    ) -> redis::RedisResult<()> {
        let k = Self::eui_key(eui);
        let k = match Self::load_addr_key(&k, conn).await? {
            Some(k) => k,
            None => {
                warn!("Error loading addr key {}", k);
                return Ok(());
            }
        };
        if redis::Cmd::exists(&k).query_async(conn).await? {
            redis::cmd("HDEL").arg(&k).arg(key).query_async(conn).await?;
        }
        Ok(())
    }
    #[instrument(skip(conn, v))]
    pub async fn update_by_eui<C: redis::aio::ConnectionLike, V: redis::ToRedisArgs>(
        eui: Eui,
        key: &str,
        v: V,
        conn: &mut C,
    ) -> redis::RedisResult<()> {
        let k = Self::eui_key(eui);
        let k = match Self::load_addr_key(&k, conn).await? {
            Some(k) => k,
            None => {
                warn!("Error loading addr key {}", k);
                return Ok(());
            }
        };
        if redis::Cmd::exists(&k).query_async(conn).await? {
            redis::cmd("HSET").arg(&k).arg(key).arg(v).query_async(conn).await?;
        }
        Ok(())
    }

    #[instrument(skip(conn))]
    async fn register<C: redis::aio::ConnectionLike>(
        &self,
        dev_eui: Eui,
        addr: LoRaAddr,
        conn: &mut C,
    ) -> redis::RedisResult<()> {
        let k1 = Self::eui_key(dev_eui);
        let k2 = Self::addr_key(addr);
        let _: () = redis::Cmd::set(&k1, &k2).query_async(conn).await?;
        redis::cmd("HSET").arg(&k2).arg(self).query_async(conn).await
    }

    #[instrument(skip(conn))]
    pub async fn check_eui<C: redis::aio::ConnectionLike>(
        dev_eui: Eui,
        conn: &mut C,
    ) -> redis::RedisResult<bool> {
        let k1 = Self::eui_key(dev_eui);
        redis::Cmd::exists(&k1).query_async(conn).await
    }

    pub async fn check_addr<C: redis::aio::ConnectionLike>(
        addr: LoRaAddr,
        conn: &mut C,
    ) -> redis::RedisResult<bool> {
        let k1 = Self::addr_key(addr);
        redis::Cmd::exists(&k1).query_async(conn).await
    }

    #[instrument(skip(conn))]
    pub async fn unregister<C: redis::aio::ConnectionLike>(
        dev_eui: Eui,
        addr: LoRaAddr,
        conn: &mut C,
    ) -> redis::RedisResult<()> {
        let k1 = Self::eui_key(dev_eui);
        let k2 = Self::addr_key(addr);
        let _: () = redis::Cmd::del(&k1).query_async(conn).await?;
        let _: () = redis::Cmd::del(&k2).query_async(conn).await?;
        Ok(())
    }

    #[instrument(skip(conn))]
    pub async fn load_by_eui<C: redis::aio::ConnectionLike>(
        dev_eui: Eui,
        conn: &mut C,
    ) -> redis::RedisResult<Option<Self>> {
        let k = Self::eui_key(dev_eui);
        match Self::load_addr_key(&k, conn).await? {
            Some(k) => redis::cmd("HGETALL")
                .arg(&k)
                .query_async::<MyOption<Self>>(conn)
                .await
                .map(Into::into),
            None => Ok(None),
        }
    }

    pub async fn load_active_time<C: redis::aio::ConnectionLike>(
        addr: LoRaAddr,
        con: &mut C,
    ) -> redis::RedisResult<Option<Timestamp>> {
        let k = Self::addr_key(addr);
        redis::Cmd::hget(&k, Self::active_time()).query_async(con).await
    }
    pub async fn load_battery<C: redis::aio::ConnectionLike>(
        addr: LoRaAddr,
        con: &mut C,
    ) -> redis::RedisResult<Option<i16>> {
        let k = Self::addr_key(addr);
        redis::Cmd::hget(&k, Self::battery()).query_async(con).await
    }
    pub async fn load_charge<C: redis::aio::ConnectionLike>(
        addr: LoRaAddr,
        con: &mut C,
    ) -> redis::RedisResult<Option<bool>> {
        let k = Self::addr_key(addr);
        redis::Cmd::hget(&k, Self::charge()).query_async(con).await
    }
    #[instrument(skip(conn))]
    async fn load_addr_key<C: redis::aio::ConnectionLike>(
        key: &str,
        conn: &mut C,
    ) -> redis::RedisResult<Option<String>> {
        let k: Option<String> = redis::cmd("GET").arg(key).query_async(conn).await?;
        Ok(k)
    }

    #[instrument(skip(conn))]
    pub async fn load_by_addr<C: redis::aio::ConnectionLike>(
        addr: LoRaAddr,
        conn: &mut C,
    ) -> redis::RedisResult<Option<Self>> {
        let k = Self::addr_key(addr);
        redis::cmd("HGETALL").arg(&k).query_async::<MyOption<Self>>(conn).await.map(Into::into)
    }

    pub async fn register_to_redis<C: redis::aio::ConnectionLike>(
        node: DeviceLoraNodeModel,
        device: DevicesModel,
        conn: &mut C,
    ) -> redis::RedisResult<Self> {
        let node_info = NodeInfo {
            user_id: Some(device.creator),
            device_id: node.device_id,
            region: node.region,
            join_type: node.join_type,
            app_eui: node.app_eui,
            dev_eui: node.dev_eui,
            app_key: node.app_key,
            dev_addr: node.dev_addr,
            nwk_skey: node.nwk_skey,
            app_skey: node.app_skey,
            class_b: node.class_b,
            class_c: node.class_c,
            adr: node.adr,
            rx1_delay: node.rx1_delay,
            des_rx1_delay: node.des_rx1_delay,
            rx1_dro: node.rx1_dro,
            des_rx1_dro: node.des_rx1_dro,
            rx2_dr: node.rx2_dr,
            des_rx2_dr: node.des_rx2_dr,
            rx2_freq: node.rx2_freq,
            des_rx2_freq: node.des_rx2_freq,
            d_retry: node.d_retry,
            c_retry: node.c_retry,
            product_type: node.product_type,
            dutycyle: node.dutycyle,
            up_confirm: node.up_confirm,
            up_dr: node.up_dr,
            up_count: 0,
            down_count: 0,
            power: node.power,
            battery: node.battery,
            charge: node.charge,
            time_zone: node.time_zone,
            firmware: node.firmware,
            dev_non: node.dev_non,
            app_non: node.app_non,
            net_id: node.net_id,
            enable: device.enable,
            period: device.period,
            script: device.script,
            active_time: device.active_time,
            gateway: None,
        };
        node_info.register(node.dev_eui, node.dev_addr, conn).await?;
        Ok(node_info)
    }
}
