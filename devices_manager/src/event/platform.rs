use crate::man::redis_client::RedisClient;
use crate::DeviceResult;
use common_define::event::PlatformLog;
use redis::AsyncCommands;
use tracing::{error, warn};

pub async fn publish_log(mut rx: tokio::sync::mpsc::Receiver<String>) {
    loop {
        match _send(&mut rx).await {
            Ok(_) => {
                warn!("log channel closed");
                return;
            }
            Err(e) => {
                error!("log to redis: {}", e)
            }
        }
    }
}

async fn _send(rx: &mut tokio::sync::mpsc::Receiver<String>) -> DeviceResult {
    let mut conn = RedisClient::get_client().get_multiplexed_conn().await?;
    while let Some(log) = rx.recv().await {
        conn.publish(PlatformLog::TOPIC, log).await?;
    }
    Ok(())
}
