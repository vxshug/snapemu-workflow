use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel,
    ModelTrait, QueryFilter,
};
use std::collections::{BTreeSet, HashMap};

use super::device::DeviceResp;
use crate::cache::{DeviceGroupCache, DeviceGroupCacheItem};
use crate::error::{ApiError, ApiResult};
use crate::service::device::order::DeviceOrderService;
use crate::service::device::DeviceService;
use crate::service::lorawan::LoRaNodeService;
use crate::service::snap::SnapDeviceService;
use crate::{get_current_user, AppState, CurrentUser, GLOBAL_PRODUCT_NAME};
use common_define::db::{
    DeviceGroupActiveModel, DeviceGroupColumn, DeviceGroupEntity, DeviceGroupModel,
    DeviceMapGroupActiveModel, DeviceMapGroupColumn, DeviceMapGroupEntity, DeviceMapGroupModel,
    DevicesEntity,
};
use common_define::product::DeviceType;
use common_define::time::Timestamp;
use common_define::Id;
use device_info::lorawan::{GatewayInfo, NodeInfo};
use tracing::log::warn;
use tracing::{debug, instrument};

pub(crate) struct DeviceGroupService;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct ReqDeviceGroup {
    pub(crate) name: String,
    pub(crate) description: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub(crate) struct ReqPutDeviceGroup {
    devices: Option<Vec<Id>>,
    remove: Option<Vec<Id>>,
    name: Option<String>,
    description: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub(crate) struct DeviceGroupResp {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) id: Option<Id>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) device_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) online_device_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) node_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) gateway_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) default_group: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) offset: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) create_time: Option<Timestamp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) devices: Option<Vec<DeviceResp>>,
}

#[inline]
pub fn cmp_period(period: i32, now: u64, active: &Option<Timestamp>) -> bool {
    match active {
        None => false,
        Some(active) => (now - active.timestamp_millis()) < ((period as u64) * 2000),
    }
}

impl DeviceGroupService {
    async fn create<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        group: ReqDeviceGroup,
        req_user: &CurrentUser,
        default: bool,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult<DeviceGroupModel> {
        let model = DeviceGroupActiveModel {
            id: Default::default(),
            name: ActiveValue::Set(group.name),
            description: ActiveValue::Set(group.description),
            default_group: ActiveValue::Set(default),
            owner: ActiveValue::Set(req_user.id),
            create_time: ActiveValue::Set(Timestamp::now()),
        };
        let ret = model.insert(conn).await?;
        DeviceGroupCache::delete_by_user_id(req_user.id, redis).await?;
        Ok(ret)
    }

    #[instrument(skip(req_user, conn))]
    pub(crate) async fn query_by_device<C: ConnectionTrait>(
        device: Id,
        req_user: &CurrentUser,
        conn: &C,
    ) -> ApiResult<Vec<DeviceGroupModel>> {
        let v = DeviceMapGroupEntity::find()
            .filter(
                DeviceMapGroupColumn::UserId
                    .eq(req_user.id)
                    .and(DeviceMapGroupColumn::DeviceId.eq(device)),
            )
            .find_also_related(DeviceGroupEntity)
            .all(conn)
            .await?
            .into_iter()
            .filter_map(|it| it.1)
            .collect();
        Ok(v)
    }
    #[instrument(skip(req_user, conn, redis))]
    pub(crate) async fn create_group<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        group: ReqDeviceGroup,
        req_user: &CurrentUser,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult<DeviceGroupModel> {
        Self::create(group, req_user, false, redis, conn).await
    }
    #[instrument(skip(conn, redis))]
    pub(crate) async fn create_group_default<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        group: ReqDeviceGroup,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult<DeviceGroupModel> {
        let user = get_current_user();
        Self::create(group, &user, true, redis, conn).await
    }
    #[instrument(skip(conn, redis))]
    pub(crate) async fn link<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        group: Id,
        user: Id,
        device: Id,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult<DeviceMapGroupModel> {
        DeviceGroupEntity::find_by_id(group)
            .one(conn)
            .await?
            .ok_or_else(|| ApiError::User("group invalid".into()))?;
        DeviceGroupCache::delete_by_user_id(user, redis).await?;
        DeviceMapGroupActiveModel {
            id: Default::default(),
            user_id: ActiveValue::Set(user),
            device_id: ActiveValue::Set(device),
            group_id: ActiveValue::Set(group),
            dev_order: ActiveValue::Set(0),
            create_time: ActiveValue::Set(Timestamp::now()),
        }
        .insert(conn)
        .await
        .map_err(Into::into)
    }

    #[instrument(skip(conn, redis))]
    pub(crate) async fn link_to_default_group<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        user: Id,
        device: Id,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult<DeviceMapGroupModel> {
        let default_group = Self::query_user_default_group(user, redis, conn).await?;
        DeviceGroupCache::delete_by_user_id(user, redis).await?;
        DeviceMapGroupActiveModel {
            id: Default::default(),
            user_id: ActiveValue::Set(user),
            device_id: ActiveValue::Set(device),
            group_id: ActiveValue::Set(default_group.id),
            dev_order: ActiveValue::Set(0),
            create_time: ActiveValue::Set(Timestamp::now()),
        }
        .insert(conn)
        .await
        .map_err(Into::into)
    }

    #[instrument(skip(conn, redis))]
    pub(crate) async fn unlink<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        device: Id,
        user: Id,
        group: Id,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult {
        DeviceGroupCache::delete_by_user_id(user, redis).await?;
        DeviceMapGroupEntity::delete_many()
            .filter(
                DeviceMapGroupColumn::DeviceId
                    .eq(device)
                    .and(DeviceMapGroupColumn::UserId.eq(user))
                    .and(DeviceMapGroupColumn::GroupId.eq(group)),
            )
            .exec(conn)
            .await?;
        Ok(())
    }

    #[instrument(skip(conn, redis))]
    pub(crate) async fn unlink_device_all<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        device: Id,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult {
        DeviceMapGroupEntity::delete_many()
            .filter(DeviceMapGroupColumn::DeviceId.eq(device))
            .exec(conn)
            .await?;
        Ok(())
    }

    pub(crate) async fn query_user_default_group<
        C: ConnectionTrait,
        R: redis::aio::ConnectionLike,
    >(
        user_id: Id,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult<DeviceGroupModel> {
        if let Some(group) = DeviceGroupCache::load_default_by_user_id(user_id, redis).await? {
            return Ok(group.group);
        }

        let default_group = DeviceGroupEntity::find()
            .filter(
                DeviceGroupColumn::Owner.eq(user_id).and(DeviceGroupColumn::DefaultGroup.eq(true)),
            )
            .one(conn)
            .await?;
        match default_group {
            Some(default_group) => Ok(default_group),
            None => DeviceGroupActiveModel {
                id: Default::default(),
                name: ActiveValue::Set("All".to_string()),
                description: ActiveValue::Set("All".to_string()),
                default_group: ActiveValue::Set(true),
                owner: ActiveValue::Set(user_id),
                create_time: ActiveValue::Set(Timestamp::now()),
            }
            .insert(conn)
            .await
            .map_err(Into::into),
        }
    }

    #[instrument(skip(conn, user, redis))]
    pub(crate) async fn update_group<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        group_id: Id,
        user: &CurrentUser,
        group_info: ReqPutDeviceGroup,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult {
        let group = DeviceGroupEntity::find_by_id(group_id)
            .one(conn)
            .await?
            .ok_or(ApiError::User("Not Found group".into()))?;
        if group.owner != user.id {
            return Err(ApiError::User("Not Found group".into()));
        }
        if group.default_group {
            return Err(ApiError::User("default is disable edit".into()));
        }

        if let Some(devices) = group_info.devices {
            let devices_set = BTreeSet::from_iter(devices.clone());
            let user_devices =
                DeviceService::query_all_with_ids(user.id, devices.as_slice(), conn).await?;
            let user_device_ids: BTreeSet<_> = user_devices.iter().map(|it| it.id).collect();
            if devices_set != user_device_ids {
                return Err(ApiError::User("invalid device".into()));
            }
            for id in devices {
                Self::link(group.id, user.id, id, redis, conn).await?;
            }
        }

        if let Some(devices) = group_info.remove {
            let devices_set = BTreeSet::from_iter(devices.clone());
            let user_devices =
                DeviceService::query_all_with_ids(user.id, devices.as_slice(), conn).await?;
            let user_device_ids: BTreeSet<_> = user_devices.iter().map(|it| it.id).collect();
            if devices_set != user_device_ids {
                return Err(ApiError::User("invalid device".into()));
            }
            for id in devices {
                Self::unlink(id, user.id, group.id, redis, conn).await?;
            }
        }

        let mut group_active = group.clone().into_active_model();
        if let Some(name) = group_info.name {
            if name != group.name {
                group_active.name = ActiveValue::Set(name);
            }
        }
        if let Some(description) = group_info.description {
            if description != group.description {
                group_active.description = ActiveValue::Set(description);
            }
        }
        if group_active.is_changed() {
            group_active.update(conn).await?;
        }

        Ok(())
    }

    #[instrument(skip(req_user, conn))]
    pub(crate) async fn select_all<C: ConnectionTrait>(
        req_user: &CurrentUser,
        conn: &C,
    ) -> ApiResult<Vec<DeviceGroupResp>> {
        let groups = DeviceGroupEntity::find()
            .filter(DeviceGroupColumn::Owner.eq(req_user.id))
            .find_with_related(DevicesEntity)
            .all(conn)
            .await?;

        let mut v = Vec::with_capacity(groups.len());
        for (group, devices) in groups {
            v.push(DeviceGroupResp {
                id: group.id.into(),
                name: group.name.into(),
                default_group: group.default_group.into(),
                device_count: Some(devices.len() as _),
                description: group.description.into(),
                create_time: None,
                devices: None,
                offset: None,
                ..Default::default()
            });
        }
        Ok(v)
    }

    #[instrument(skip(req_user, redis, state))]
    pub(crate) async fn select_default_group<R: redis::aio::ConnectionLike>(
        req_user: &CurrentUser,
        redis: &mut R,
        state: &AppState,
    ) -> ApiResult<DeviceGroupResp> {
        let conn = &state.db;
        let group = {
            let cache = DeviceGroupCache::load_default_by_user_id(req_user.id, redis).await?;
            match cache {
                Some(group) => group,
                None => {
                    let default_group = DeviceGroupEntity::find()
                        .filter(
                            DeviceGroupColumn::Owner
                                .eq(req_user.id)
                                .and(DeviceGroupColumn::DefaultGroup.eq(true)),
                        )
                        .one(conn)
                        .await?;
                    match default_group {
                        None => {
                            let group =
                                ReqDeviceGroup { name: "All".into(), description: "All".into() };
                            let default_group =
                                DeviceGroupService::create_group_default(group, redis, conn)
                                    .await?;
                            let groups = DeviceGroupEntity::find()
                                .filter(DeviceGroupColumn::Owner.eq(req_user.id))
                                .find_with_related(DevicesEntity)
                                .all(conn)
                                .await?;
                            let caches: Vec<_> = groups
                                .into_iter()
                                .map(|it| DeviceGroupCacheItem::new(it.0, it.1.len() as _))
                                .collect();
                            DeviceGroupCache::save_by_user_id(
                                DeviceGroupCache::new(caches),
                                req_user.id,
                                redis,
                            )
                            .await?;
                            debug!("create default group: {}", default_group.id);
                            return Ok(DeviceGroupResp {
                                id: default_group.id.into(),
                                name: default_group.name.into(),
                                device_count: Some(0),
                                default_group: default_group.default_group.into(),
                                offset: Some(0),
                                description: default_group.description.into(),
                                create_time: default_group.create_time.into(),
                                devices: Some(Vec::new()),
                                ..Default::default()
                            });
                        }
                        Some(o) => DeviceGroupCacheItem::new(o, 0),
                    }
                }
            }
        };
        Self::select_one(req_user, state, group).await
    }

    #[instrument(skip(req_user, redis, conn))]
    pub(crate) async fn select_by_group_id<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        req_user: &CurrentUser,
        group_id: Id,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult<DeviceGroupCacheItem> {
        let caches = DeviceGroupCache::load_by_user_id(req_user.id, redis).await?;
        if let Some(caches) = caches {
            for cache in caches.v {
                if cache.group.id == group_id {
                    debug!("select group by id {} in cache", cache.group.id);
                    return Ok(cache);
                }
            }
        }

        debug!("select group by id {} in cache missing", group_id);
        let groups = DeviceGroupEntity::find()
            .filter(DeviceGroupColumn::Owner.eq(req_user.id))
            .find_with_related(DevicesEntity)
            .all(conn)
            .await?;
        let caches: Vec<_> =
            groups.into_iter().map(|it| DeviceGroupCacheItem::new(it.0, it.1.len() as _)).collect();

        debug!("select group by id {} updated cache", group_id);
        DeviceGroupCache::save_by_user_id(
            DeviceGroupCache::new(caches.clone()),
            req_user.id,
            redis,
        )
        .await?;

        for cache in caches {
            if cache.group.id == group_id {
                return Ok(cache);
            }
        }
        warn!("group id `{}` not found`", group_id);
        Err(ApiError::User("group not found".into()))
    }
    #[instrument(skip(req_user, state))]
    pub(crate) async fn select_one(
        req_user: &CurrentUser,
        state: &AppState,
        group: DeviceGroupCacheItem,
    ) -> ApiResult<DeviceGroupResp> {
        let conn = &state.db;
        let redis = &mut state.redis.get().await?;

        let order_device = DeviceService::query_all(req_user.id, redis, conn).await?;

        let device_order_save: Vec<_> = order_device.iter().map(|item| item.id).collect();
        let functions =
            DeviceService::query_all_device_functions(device_order_save.clone(), conn).await?;
        let mut blue_function: HashMap<Id, String> = functions
            .into_iter()
            .filter(|it| it.func_name == DeviceService::FUNC_BLUETOOTH)
            .map(|it| (it.device, it.func_value))
            .collect();

        let mut node_device = vec![];
        let orders = DeviceOrderService::device_order_group(group.group.id, conn).await?;
        let mut snap_device = vec![];
        let mut gateway_device_count = 0i64;
        for device in &order_device {
            match device.device_type {
                DeviceType::LoRaNode => node_device.push(device.id),
                DeviceType::LoRaGate => gateway_device_count += 1,
                DeviceType::MQTT => (),
                DeviceType::Snap => snap_device.push(device.id),
            }
        }
        let online_time =
            DeviceService::query_device_online_with_ids(node_device.as_slice(), conn).await?;
        let mut online_device_count = 0;
        let node_device_count = node_device.len() as i64;
        let mut last_data = Vec::with_capacity(node_device.len() + snap_device.len());
        last_data.extend_from_slice(node_device.as_slice());
        last_data.extend_from_slice(snap_device.as_slice());
        let node_device = LoRaNodeService::get_all_lora_node(node_device, conn).await?;
        let mut node_device: HashMap<_, _> =
            node_device.into_iter().map(|item| (item.device_id, item)).collect();

        let snap_device = SnapDeviceService::get_all(snap_device, conn).await?;
        let mut snap_device: HashMap<_, _> =
            snap_device.into_iter().map(|item| (item.device_id, item)).collect();

        let mut last_data = DeviceService::query_last_data(order_device.as_slice(), state).await?;

        let mut order_device: HashMap<_, _> =
            order_device.into_iter().map(|item| (item.id, item)).collect();
        let mut v = Vec::with_capacity(order_device.len());

        let now_time = Timestamp::now().timestamp_millis();

        for order in orders {
            if let Some(device) = order_device.remove(&order.device_id) {
                let product = GLOBAL_PRODUCT_NAME.get_by_id(device.product_id);
                let (product_id, product_url) = match product {
                    Some(p) => (Some(p.id), Some(p.image)),
                    None => (None, None),
                };
                match device.device_type {
                    DeviceType::Snap => {
                        let data = last_data.remove(&device.id);
                        if let Some(node) = snap_device.remove(&device.id) {
                            v.push(DeviceResp {
                                id: device.id.into(),
                                eui: Some(device.eui),
                                name: device.name.into(),
                                blue_name: blue_function.remove(&device.id),
                                online: Some(cmp_period(device.period, now_time, &device.active_time)),
                                period: None,
                                charge: node.charge.into(),
                                battery: node.battery,
                                description: device.description.into(),
                                info: None,
                                source: None,
                                device_type: device.device_type.into(),
                                product_type: None,
                                create_time: None,
                                active_time: device.active_time,
                                data,
                                script: None,
                                product_id,
                                product_url,
                                group: None,
                            })
                        }
                    }
                    DeviceType::MQTT => v.push(DeviceResp {
                        id: device.id.into(),
                        eui: Some(device.eui),
                        name: device.name.into(),
                        blue_name: blue_function.remove(&device.id),
                        online: Some(cmp_period(device.period, now_time, &device.active_time)),
                        period: None,
                        charge: None,
                        battery: None,
                        description: device.description.into(),
                        info: None,
                        source: None,
                        device_type: device.device_type.into(),
                        product_type: None,
                        create_time: None,
                        active_time: device.active_time,
                        data: None,
                        script: None,
                        product_id,
                        product_url,
                        group: None,
                    }),
                    DeviceType::LoRaNode => {
                        let data = last_data.remove(&device.id);
                        if let Some(node) = node_device.remove(&device.id) {
                            let active_time = device.active_time;
                            if let Some(active_time) = active_time {
                                let time = Timestamp::now().timestamp_millis()
                                    - active_time.timestamp_millis();
                                if let Some(online_time) = online_time.get(&device.id) {
                                    if time < *online_time {
                                        online_device_count += 1;
                                    }
                                }
                            }
                            v.push(DeviceResp {
                                id: device.id.into(),
                                eui: Some(device.eui),
                                name: device.name.into(),
                                blue_name: blue_function.remove(&device.id),
                                online: Some(cmp_period(device.period, now_time, &device.active_time)),
                                period: None,
                                charge: NodeInfo::load_charge(node.dev_addr, redis).await?,
                                battery: NodeInfo::load_battery(node.dev_addr, redis).await?,
                                description: device.description.into(),
                                info: None,
                                source: None,
                                device_type: device.device_type.into(),
                                product_type: None,
                                create_time: None,
                                active_time,
                                data,
                                script: None,
                                product_id,
                                product_url,
                                group: None,
                            })
                        }
                    }
                    DeviceType::LoRaGate => v.push(DeviceResp {
                        id: device.id.into(),
                        eui: Some(device.eui),
                        name: device.name.into(),
                        blue_name: None,
                        online: Some(cmp_period(device.period, now_time, &device.active_time)),
                        period: None,
                        charge: None,
                        battery: None,
                        description: device.description.into(),
                        info: None,
                        source: None,
                        device_type: device.device_type.into(),
                        product_type: None,
                        create_time: None,
                        active_time: device.active_time,
                        data: None,
                        script: None,
                        product_id,
                        product_url,
                        group: None,
                    }),
                }
            }
        }

        let group = DeviceGroupResp {
            id: group.group.id.into(),
            name: group.group.name.into(),
            default_group: group.group.default_group.into(),
            device_count: (v.len() as i64).into(),
            offset: Some(0),
            description: group.group.description.into(),
            create_time: None,
            devices: v.into(),
            gateway_count: Some(gateway_device_count),
            node_count: Some(node_device_count),
            online_device_count: Some(online_device_count),
            ..Default::default()
        };
        Ok(group)
    }

    #[instrument(skip(user, conn, redis))]
    pub(crate) async fn delete_one<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        user: &CurrentUser,
        id: Id,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult {
        let device_group = DeviceGroupEntity::find_by_id(id)
            .one(conn)
            .await?
            .ok_or(ApiError::User("Notfound".into()))?;
        if device_group.owner != user.id {
            return Err(ApiError::User("Notfound".into()));
        }

        if device_group.default_group {
            Err(ApiError::User("The default group cannot be deleted".into()))
        } else {
            device_group.delete(conn).await?;
            Ok(())
        }
    }

    #[instrument(skip(conn, redis))]
    pub(crate) async fn delete_by_user<C: ConnectionTrait, R: redis::aio::ConnectionLike>(
        user_id: Id,
        redis: &mut R,
        conn: &C,
    ) -> ApiResult {
        DeviceGroupEntity::delete_many()
            .filter(DeviceGroupColumn::Owner.eq(user_id))
            .exec(conn)
            .await?;
        DeviceMapGroupEntity::delete_many()
            .filter(DeviceMapGroupColumn::UserId.eq(user_id))
            .exec(conn)
            .await?;
        DeviceGroupCache::delete_by_user_id(user_id, redis).await?;
        Ok(())
    }
}
