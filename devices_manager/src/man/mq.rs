use crate::man::redis_client::RedisRecv;
use crate::service::gateway_statue::{gateway_status, PushState};
use crate::service::lorawan_node::PushData;
use crate::{DeviceError, DeviceResult, GLOBAL_TOPIC};
use tokio_stream::StreamExt;
use tracing::{debug, debug_span, error, instrument, Instrument};
use tracing::{info, warn};

pub struct MQ {
    recv: RedisRecv,
}

impl MQ {
    pub async fn new(mut recv: RedisRecv) -> Self {
        recv.subscribe(GLOBAL_TOPIC.down).await.unwrap();
        Self { recv }
    }

    #[instrument(skip(self), name = "UpLink")]
    async fn recv(&mut self) -> DeviceResult {
        let mut message = self.recv.message();
        while let Some(message) = message.next().await {
            let topic = message.get_channel_name();
            let payload = message.get_payload_bytes();
            if GLOBAL_TOPIC.data == topic {
                if let Err(e) = node_up_data(payload).await {
                    warn!("{e}")
                }
            }

            if GLOBAL_TOPIC.gate_event == topic {
                if let Err(e) = gateway_up_event(payload).await {
                    warn!("{e}")
                }
            }
        }
        Ok(())
    }

    pub async fn start(&mut self) {
        self.recv.subscribe(GLOBAL_TOPIC.data).await.unwrap();
        self.recv.subscribe(GLOBAL_TOPIC.gate_event).await.unwrap();
        info!("start recv message");
        loop {
            if let Err(e) = self.recv().await {
                error!("{}", e);
                // match e {
                //     DeviceError::Json(_) => {}
                //     DeviceError::Data(_) => {}
                //     DeviceError::Device(_) => {}
                //     DeviceError::Warn(_) => {}
                //     DeviceError::Base64(_) => {}
                //     DeviceError::Redis(_) => {}
                //     DeviceError::Hex(_) => {}
                //     DeviceError::Db(_) => {}
                //
                //     DeviceError::Error(_) => {}
                //     DeviceError::Empty => {}
                // }
            }
        }
    }
}

async fn gateway_up_event(payload: &[u8]) -> DeviceResult {
    let state: PushState = serde_json::from_slice(payload)?;
    let s = std::str::from_utf8(payload);
    let span = debug_span!(
        "node_up_data",
        gateway_eui = state.eui.to_string(),
        gateway_id = state.ip.to_string()
    );
    span.in_scope(|| {
        debug!("{:?}", s);
    });
    async {
        if let Err(e) = gateway_status(state).await {
            warn!("{e}")
        }
    }
    .instrument(span)
    .await;
    Ok(())
}

async fn node_up_data(payload: &[u8]) -> DeviceResult {
    let push: PushData = serde_json::from_slice(payload)?;
    let s = std::str::from_utf8(payload).map_err(|e| DeviceError::Data(e.to_string()))?;
    let span = debug_span!(
        "node_up_data",
        gateway_eui = push.eui.to_string(),
        gateway_id = push.gateway.to_string()
    );
    span.in_scope(|| {
        debug!("{}", s);
    });

    // tokio::spawn(
    //     node_data(push.eui, push.gateway, push.pk.rssi, push)
    // );

    Ok(())
}
