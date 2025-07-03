use crate::error::ApiResult;
use crate::RedisConnection;
use common_define::db::{DeviceLoraGateEntity, DeviceLoraNodeEntity, DevicesEntity};
use common_define::time::Timestamp;
use device_info::lorawan::{GatewayInfo, NodeInfo};
use sea_orm::{ConnectionTrait, EntityTrait};
use tracing::info;

pub async fn sync_device<C: ConnectionTrait>(
    conn: &C,
    redis_conn: &mut RedisConnection,
) -> ApiResult {
    info!("sync device");
    sync_node(conn, redis_conn).await?;
    sync_gateway(conn, redis_conn).await?;
    info!("sync device ok");
    Ok(())
}

pub async fn sync_gateway<C: ConnectionTrait>(
    conn: &C,
    redis_conn: &mut RedisConnection,
) -> ApiResult {
    let gateways = DeviceLoraGateEntity::find().all(conn).await?;

    for gateway in gateways {
        let exist = GatewayInfo::check_eui(gateway.eui, redis_conn).await?;
        if !exist {
            GatewayInfo::new(gateway.device_id, 0, 0, Timestamp::now(), None, None)
                .register(gateway.eui, redis_conn)
                .await?;
        }
    }
    Ok(())
}

pub async fn sync_node<C: ConnectionTrait>(
    conn: &C,
    redis_conn: &mut RedisConnection,
) -> ApiResult {
    let nodes = DeviceLoraNodeEntity::find()
        .find_also_related(DevicesEntity)
        .all(conn)
        .await?
        .into_iter()
        .filter_map(|it| match it.1 {
            Some(device) => Some((device, it.0)),
            None => None,
        })
        .collect::<Vec<_>>();

    for (device, node) in nodes {
        let exist = NodeInfo::check_eui(node.dev_eui, redis_conn).await?;
        if !exist {
            NodeInfo::register_to_redis(node, device, redis_conn).await?;
        }
    }
    Ok(())
}
