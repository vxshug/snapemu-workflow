use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::Duration;

use common_define::db::{
    DeviceLoraGateColumn, DeviceLoraGateEntity, DeviceLoraNodeColumn, DeviceLoraNodeEntity,
    DevicesEntity, Eui, Key, LoRaAddr,
};
use common_define::lora::LoRaRegion;
use common_define::lorawan_bridge::{DownStream, GatewayToken};
use common_define::time::Timestamp;
use device_info::lorawan::{GatewayInfo, NodeInfo};
use lorawan::parser::DataHeader;
use once_cell::sync::Lazy;
use redis::AsyncCommands;
use sea_orm::ColumnTrait;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use tracing::{debug, error, info, instrument, warn};

use super::Id;
use crate::event::DeviceManagerServer;
use crate::man::data::DownloadData;
use crate::man::redis_client::RedisClient;
use crate::protocol::lora::payload::LoRaPayload;
use crate::{
    protocol::lora::{
        self,
        data::{JoinRespDataBuilder, RespDataBuilder, RespDataClassCBuilder},
    },
    service::lorawan_node::PushData,
    DeviceError, DeviceResult, GLOBAL_DOWNLOAD, GLOBAL_STATE,
};

#[derive(
    derive_more::From,
    Debug,
    Clone,
    Copy,
    serde::Serialize,
    serde::Deserialize,
    redis_macros::FromRedisValue,
    redis_macros::ToRedisArgs
)]
#[serde(transparent)]
pub(crate) struct LoRaRegionLocal(pub LoRaRegion);

struct GatewayTimeline(Mutex<HashMap<Eui, u64>>);

const TIME_DURATION: u64 = 2000000;

impl GatewayTimeline {
    fn get_time(&self, gateway: Eui) -> Duration {
        let mut g = self.0.lock().unwrap();
        let now = Timestamp::now().timestamp_micros();
        match g.get_mut(&gateway) {
            None => {
                g.insert(gateway, now + TIME_DURATION);
                Duration::from_secs(0)
            }
            Some(g) => {
                let pre = *g;
                if pre < now {
                    *g = now + TIME_DURATION;
                    Duration::from_secs(0)
                } else {
                    *g = pre + TIME_DURATION;
                    let n = pre - now;
                    Duration::from_micros(n)
                }
            }
        }
    }
}

static GATEWAY_TIME: Lazy<GatewayTimeline> = Lazy::new(|| GatewayTimeline(Default::default()));

#[derive(
    Debug,
    redis_macros::ToRedisArgs,
    redis_macros::FromRedisValue,
    serde::Serialize,
    serde::Deserialize
)]
pub(crate) struct LoRaOTAANodeInfo {
    pub(crate) app_skey: Key,
    pub(crate) nwk_skey: Key,
    pub(crate) dev_nonce: u16,
    pub(crate) app_nonce: u32,
    pub(crate) net_id: u32,
}

pub(crate) struct LoRaNode {
    pub key: String,
    task_key: String,
    pub gw: LoRaGate,
    pub(crate) info: NodeInfo,
    conn: redis::aio::MultiplexedConnection,
}

impl LoRaNode {
    fn keys(dev_addr: LoRaAddr) -> (String, String) {
        (format!("lora:node:{}", dev_addr), format!("lora:tasks:{}", dev_addr))
    }
    fn dev_keys(dev_addr: &str) -> String {
        format!("lora:node:{}", dev_addr)
    }
    fn task_key(dev_addr: LoRaAddr) -> String {
        format!("lora:tasks:{}", dev_addr)
    }
    fn activate_key(dev_addr: LoRaAddr) -> String {
        format!("lora:otaa:{}", dev_addr)
    }
    fn id_keys(id: Eui) -> String {
        format!("lora:node:{}", id)
    }

    #[instrument(skip_all)]
    async fn down_link(&self, down: DownStream) -> DeviceResult {
        self.gw.down_link(down).await?;
        Ok(())
    }
    pub(crate) async fn update_up_count(&mut self, up_count: u32) -> DeviceResult {
        NodeInfo::update_by_addr(
            self.info.dev_addr,
            NodeInfo::up_count(),
            up_count,
            &mut self.conn,
        )
        .await?;
        Ok(())
    }
    pub(crate) async fn update_time(&mut self) -> DeviceResult {
        NodeInfo::update_by_addr(
            self.info.dev_addr,
            NodeInfo::active_time(),
            Timestamp::now(),
            &mut self.conn,
        )
        .await?;
        Ok(())
    }
    pub(crate) async fn dispatch_task(&self, mut task: DownloadData) -> DeviceResult {
        if self.info.class_c {
            let _ = task.up_count.insert(self.info.up_count);
            if let Some(gateway_eui) = self.info.gateway {
                let wait = GATEWAY_TIME.get_time(gateway_eui);
                tracing::info!(gateway = gateway_eui.to_string(), "gateway busy wait {:?}", wait);
                tokio::time::sleep(wait).await;
                let gateway = LoRaGateManager::get_gate(gateway_eui).await?;
                let info = gateway.info().await?;
                let builder = RespDataClassCBuilder::new(&self.info, &info);
                let re_data = builder.build_with_task(&task, rand::random())?;
                tracing::info!(
                    gateway = gateway_eui.to_string(),
                    "Class C DownLink: {:02X?}",
                    task.bytes.as_ref()
                );
                let counter = GLOBAL_DOWNLOAD.insert(self.info.dev_eui, task.clone());
                let device_addr = self.info.dev_addr;
                let dev_eui = self.info.dev_eui;
                tokio::spawn(async move {
                    if let Err(e) = repetition_task(task, device_addr, gateway_eui, counter).await {
                        warn!(
                            target: "repetition_task",
                            device_eui = dev_eui.to_string(),
                            "{e}"
                        )
                    }
                });

                self.update_down_count().await?;
                gateway.down_link(re_data).await?;
            }
        } else {
            warn!("not is class c device");
        }
        Ok(())
    }

    pub(crate) async fn dispatch_task_now(&self, mut task: DownloadData) -> DeviceResult {
        let _ = task.up_count.insert(self.info.up_count);
        if self.info.class_c {
            if let Some(gateway_eui) = self.info.gateway {
                let gateway = LoRaGateManager::get_gate(gateway_eui).await?;
                let info = gateway.info().await?;
                let builder = RespDataClassCBuilder::new(&self.info, &info);
                let re_data = builder.build_with_task(&task, rand::random())?;
                tracing::info!(
                    gateway = gateway_eui.to_string(),
                    "Class C DownLink: {:02X?}",
                    task.bytes.as_ref()
                );

                self.update_down_count().await?;
                gateway.down_link(re_data).await?;
                return Ok(());
            }
        } else {
            warn!("not is class c device");
        }
        GLOBAL_DOWNLOAD.insert(self.info.dev_eui, task.clone());
        Ok(())
    }

    pub(crate) async fn get_otaa_info(&mut self) -> DeviceResult<Option<LoRaOTAANodeInfo>> {
        let active_key = LoRaNode::activate_key(self.info.dev_addr);
        let info: Option<LoRaOTAANodeInfo> = self.conn.get(&active_key).await?;
        if info.is_some() {
            self.conn.del(active_key).await?;
        }
        Ok(info)
    }

    async fn wait(&self) {
        let delay = if self.info.rx1_delay < 2 { 0 } else { self.info.rx1_delay - 1 };
        let wait = Duration::from_secs(delay as u64);
        info!("gateway wait: {:?}", wait);
    }
    pub(crate) async fn pull_task(
        &mut self,
        payload: &[u8],
        push_data: &PushData,
        header: &LoRaPayload,
    ) -> DeviceResult {
        let count = header.fhdr().fcnt() as u32;
        info!("DownLink start, count: {}, gateway: {:?}", count, self.gw.eui);
        let eui = self.info.dev_eui;
        let task = GLOBAL_DOWNLOAD.pop(eui);

        match task {
            Some(task) => {
                tracing::info!("down data: {:?}", task.bytes);
                let ack = false;
                let resp = task.clone();
                if let Some(pre_up_count) = task.up_count {
                    debug!("pull_task: get pre_up_count: {pre_up_count}, count: {count}");
                    self.wait().await;
                    self.update_down_count().await?;
                    let builder = RespDataBuilder::new(&self.info, push_data);
                    if ack {
                        let ack = builder.build_ack(&[])?;
                        self.down_link(ack).await?;
                    } else {
                        let down =
                            builder.build_with_task(&resp, rand::random(), push_data.version)?;
                        self.down_link(down).await?;
                    }
                    GLOBAL_STATE.event.lora_node_downlink_data(push_data, &self.info, Some(&task)).await;
                }
            }
            None => {
                let confirm = header.is_confirmed();
                debug!("DownLink ack: {}", confirm);
                self.wait().await;
                let response = {
                    if confirm {
                        let builder = RespDataBuilder::new(&self.info, push_data);
                        let ack = builder.build_ack(&[])?;
                        Some(ack)
                    } else {
                        None
                    }
                };
                if let Some(ack) = response {
                    self.down_link(ack).await?;
                    self.update_down_count().await?;
                    GLOBAL_STATE.event.lora_node_downlink_data(push_data, &self.info, None).await;
                }
            }
        }

        Ok(())
    }
    pub(crate) async fn update_down_count(&self) -> DeviceResult<u32> {
        let mut conn = self.conn.clone();
        let count = redis::cmd("HINCRBY")
            .arg(NodeInfo::addr_key(self.info.dev_addr))
            .arg(NodeInfo::down_count())
            .arg(1)
            .query_async(&mut conn)
            .await?;
        Ok(count)
    }
    pub(crate) async fn reset_down_count(&mut self) -> DeviceResult<u32> {
        NodeInfo::update_by_addr(self.info.dev_addr, NodeInfo::down_count(), 0, &mut self.conn)
            .await?;
        self.info.down_count = 0;
        Ok(0)
    }

    pub(crate) async fn update_gateway(&mut self) -> DeviceResult {
        if Some(self.gw.eui) != self.info.gateway {
            self.info.gateway = Some(self.gw.eui);
            NodeInfo::update_by_addr(
                self.info.dev_addr,
                NodeInfo::gateway(),
                self.gw.eui,
                &mut self.conn,
            )
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn update_charge(&mut self, charge: bool) -> DeviceResult {
        NodeInfo::update_by_addr(self.info.dev_addr, NodeInfo::charge(), charge, &mut self.conn)
            .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct LoRaNodeManager;

impl LoRaNodeManager {
    #[instrument(skip(gw))]
    pub(crate) async fn get_node_with_gateway(
        dev_addr: LoRaAddr,
        gw: LoRaGate,
    ) -> DeviceResult<LoRaNode> {
        let mut conn = RedisClient::get_client().get_multiplexed_conn().await?;
        let info = match NodeInfo::load_by_addr(dev_addr, &mut conn).await? {
            None => {
                let (node, devices) = DeviceLoraNodeEntity::find()
                    .filter(DeviceLoraNodeColumn::DevAddr.eq(dev_addr))
                    .find_also_related(DevicesEntity)
                    .one(&GLOBAL_STATE.db)
                    .await?
                    .ok_or_else(|| {
                        warn!("device addr({}) is not register", dev_addr);
                    })?;
                if devices.is_none() {
                    error!("dev_addr({}) in lora_node, but found in devices", dev_addr);
                    return Err(DeviceError::Empty);
                }
                NodeInfo::register_to_redis(node, devices.unwrap(), &mut conn).await?
            }
            Some(info) => info,
        };
        let key = NodeInfo::addr_key(dev_addr);
        let task_key = LoRaNode::task_key(dev_addr);
        Ok(LoRaNode { conn, key, info, task_key, gw })
    }

    #[instrument]
    pub(crate) async fn get_node(dev_addr: LoRaAddr) -> DeviceResult<LoRaNode> {
        let mut conn = RedisClient::get_client().get_multiplexed_conn().await?;
        let info = NodeInfo::load_by_addr(dev_addr, &mut conn)
            .await?
            .ok_or(DeviceError::device("device already delete"))?;
        let key = NodeInfo::addr_key(dev_addr);
        let task_key = LoRaNode::task_key(dev_addr);
        let gw = match info.gateway {
            Some(gateway) => LoRaGateManager::get_gate(gateway).await?,
            None => return Err(DeviceError::device("device not exists")),
        };
        Ok(LoRaNode { conn, key, info, task_key, gw })
    }

    pub(crate) async fn otaa_active(dev_addr: LoRaAddr) -> DeviceResult {
        let active_key = LoRaNode::activate_key(dev_addr);
        let mut conn = RedisClient::get_client().get_multiplexed_conn().await?;
        conn.del(active_key).await?;
        Ok(())
    }

    pub(crate) async fn new_otaa_node(
        data: &PushData,
        info: NodeInfo,
        dev_nonce: u16,
        gw: LoRaGate,
    ) -> DeviceResult {
        let app_nonce = rand::random::<u32>() & 0xFFFFFF;
        let net_id = rand::random::<u32>() & 0xFFFFFF;
        let keys = lora::join_accept::NodeKeys::new(&info.app_key, app_nonce, net_id, dev_nonce);
        let mut conn = RedisClient::get_client().get_multiplexed_conn().await?;

        GLOBAL_STATE.event.lora_node_join_request(data, &info).await;
        let join_builder = JoinRespDataBuilder::new(&info, data);
        let resp = join_builder.build(info.dev_addr, app_nonce, net_id, &info.app_key)?;

        let active_key = LoRaNode::activate_key(info.dev_addr);

        let otaa_info = LoRaOTAANodeInfo {
            nwk_skey: keys.nwk_skey,
            app_skey: keys.app_skey,
            dev_nonce,
            app_nonce,
            net_id,
        };
        let info_json = serde_json::to_string(&otaa_info)?;

        conn.set(active_key, info_json).await?;
        gw.down_link(resp).await?;
        GLOBAL_STATE.event.lora_node_join_accept(info.dev_addr, &info).await;
        debug!("Join Request");
        Ok(())
    }

    pub(crate) async fn delete_device(&self, id: Eui) -> DeviceResult {
        let mut conn = RedisClient::get_client().get_multiplexed_conn().await?;
        let key = LoRaNode::id_keys(id);
        let dev_addr: Option<LoRaAddr> = conn.get(&key).await?;
        if let Some(dev_addr) = dev_addr {
            let (dev_key, task_key) = LoRaNode::keys(dev_addr);
            conn.del(dev_key).await?;
            conn.del(task_key).await?;
            conn.del(key).await?;
        }
        Ok(())
    }

    pub(crate) async fn get_node_by_eui(eui: Eui) -> DeviceResult<Option<LoRaNode>> {
        let mut conn = RedisClient::get_client().get_multiplexed_conn().await?;
        let info = NodeInfo::load_by_eui(eui, &mut conn).await?;

        match info {
            Some(info) => {
                let gw = match info.gateway {
                    Some(gateway) => LoRaGateManager::get_gate(gateway).await?,
                    None => return Err(DeviceError::device("gateway not found")),
                };

                let task_key = LoRaNode::task_key(info.dev_addr);
                let key = NodeInfo::addr_key(info.dev_addr);
                Ok(Some(LoRaNode { conn, key, task_key, info, gw }))
            }
            None => Ok(None),
        }
    }
}

#[derive(Clone)]
pub(crate) struct LoRaGate {
    key: String,
    pub eui: Eui,
    pub id: Id,
    pub down: Option<SocketAddr>,
    pub(crate) info: GatewayInfo,
}

impl LoRaGate {
    pub(crate) async fn info(&self) -> DeviceResult<GatewayInfo> {
        let mut conn = RedisClient::get_client().get_multiplexed_conn().await?;
        GatewayInfo::load(self.eui, &mut conn)
            .await?
            .ok_or(DeviceError::device("gateway not found"))
    }
    pub(crate) async fn tmst(&self) -> DeviceResult<u32> {
        let mut conn = RedisClient::get_client().get_multiplexed_conn().await?;
        let (tmst, time): (u32, Timestamp) =
            conn.hget(self.key.as_str(), (GatewayInfo::tmst(), GatewayInfo::tmst())).await?;
        let now = Timestamp::now().timestamp_micros();
        let time = time.timestamp_micros();
        let tmst = ((now - time) as u32) + tmst;
        Ok(tmst)
    }
    pub(crate) async fn update_version(&mut self, version: u8) -> DeviceResult {
        if self.info.version != version {
            let mut conn = RedisClient::get_client().get_multiplexed_conn().await?;
            self.info.version = version;
            conn.hset(self.key.as_str(), GatewayInfo::version(), version).await?;
        }
        Ok(())
    }
    #[instrument(skip(self))]
    pub(crate) async fn update_down(&mut self, down: Option<SocketAddr>) -> DeviceResult {
        let addr = down.map(|addr| addr.to_string());
        if addr == self.info.down {
            return Ok(());
        }
        self.down = down;
        let mut conn = RedisClient::get_client().get_multiplexed_conn().await?;

        conn.hset(self.key.as_str(), GatewayInfo::down(), &addr).await?;
        Ok(())
    }
    #[instrument(skip(self))]
    pub(crate) async fn update_tmst(&mut self, tmst: u32) -> DeviceResult {
        self.info.tmst = tmst;
        let mut conn = RedisClient::get_client().get_multiplexed_conn().await?;
        let now = Timestamp::now();
        conn.hset(self.key.as_str(), (GatewayInfo::tmst(), tmst), (GatewayInfo::time(), now))
            .await?;
        Ok(())
    }

    async fn down_link(&self, down: DownStream) -> DeviceResult {
        GLOBAL_STATE.udp.down(down, self.info.version, GatewayToken::random(), self.down).await?;
        Ok(())
    }
    pub async fn pull_ack(&mut self, token: GatewayToken) -> DeviceResult {
        GLOBAL_STATE.udp.pull_ack(self.info.version, token, self.eui, self.down).await?;
        Ok(())
    }
    pub async fn push_ack(
        &mut self,
        token: GatewayToken,
        addr: Option<SocketAddr>,
    ) -> DeviceResult {
        GLOBAL_STATE.udp.push_ack(self.info.version, token, addr).await?;
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct LoRaGateManager;

impl LoRaGateManager {
    #[instrument]
    pub(crate) async fn get_gate(eui: Eui) -> DeviceResult<LoRaGate> {
        let mut conn = RedisClient::get_client().get_multiplexed_conn().await?;
        let mut info = match GatewayInfo::load(eui, &mut conn).await? {
            None => {
                let device = DeviceLoraGateEntity::find()
                    .filter(DeviceLoraGateColumn::Eui.eq(eui))
                    .one(&GLOBAL_STATE.db)
                    .await?
                    .ok_or_else(|| {
                        warn!("gateway not register");
                        DeviceError::device("gateway not register".to_string())
                    })?;
                debug!("active gateway");
                let info = GatewayInfo::new(device.device_id, 0, 2, Timestamp::now(), None, None);
                info.register(eui, &mut conn).await?;
                info
            }
            Some(info) => info,
        };

        let now = Timestamp::now().timestamp_micros();
        let time = info.time.timestamp_micros();
        let tmst = ((now - time) as u32).wrapping_add(info.tmst);
        info.tmst = tmst;
        let key = GatewayInfo::eui_key(eui);
        GatewayInfo::update_active_time(eui, &mut conn).await?;
        Ok(LoRaGate {
            id: info.device,
            down: info.down.clone().and_then(|s| s.parse().ok()),
            key,
            eui,
            info,
        })
    }
}

#[instrument(skip(data))]
async fn repetition_task(
    data: DownloadData,
    device_addr: LoRaAddr,
    device_eui: Eui,
    counter: u64,
) -> DeviceResult {
    let mut repetition = 0;
    loop {
        tokio::time::sleep(Duration::from_secs(6)).await;
        if GLOBAL_DOWNLOAD.repetition_task(device_eui, counter) {
            let s = LoRaNodeManager::get_node(device_addr).await?;
            let gateway = LoRaGateManager::get_gate(
                s.info.gateway.ok_or(DeviceError::Device("not found gateway eui".to_string()))?,
            )
            .await?;
            let builder = RespDataClassCBuilder::new(&s.info, &gateway.info);
            let re_data = builder.build_with_task(&data, rand::random())?;
            gateway.down_link(re_data).await?;
            s.update_down_count().await?;
        } else {
            break;
        }
        repetition += 1;
        tracing::info!("Class C DownLink: repetition_task timeout 6s at {}", repetition);
        if repetition == 10 {
            break;
        }
    }
    GLOBAL_DOWNLOAD.commit(device_eui);
    Ok(())
}
