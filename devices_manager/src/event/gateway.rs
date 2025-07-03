use crate::man::redis_client::RedisClient;
use crate::protocol::lora;
use crate::DeviceResult;
use common_define::event::lora_gateway::{GatewayEventType};
use common_define::lorawan_bridge::GatewayUpData;
use common_define::time::Timestamp;
use common_define::{lorawan_bridge, Id};
use lorawan::parser::DataHeader;
use redis::AsyncCommands;

use crate::protocol::lora::parse::LoraPhy;

pub struct GatewayEvent;
impl GatewayEvent {
    pub(crate) async fn gateway_state(gateway_id: Id, state: GatewayUpData) -> DeviceResult {
        let mut conn = RedisClient::get_client().get_multiplexed_conn().await?;
        match state.event {
            lorawan_bridge::GatewayEventType::Status(st) => {
                let resp = common_define::event::DeviceEvent {
                    device: gateway_id,
                    event: common_define::event::DeviceEventType::Gateway(
                        common_define::event::lora_gateway::GatewayEvent {
                            eui: state.eui,
                            time: Timestamp::from_timestamp_millis(state.time.timestamp_millis())
                                .unwrap_or(Timestamp::now()),
                            gateway_event: GatewayEventType::Status(
                                common_define::event::lora_gateway::GatewayStatus {
                                    time: st.time,
                                    lati: st.lati,
                                    long: st.long,
                                    alti: st.alti,
                                    rxnb: st.rxnb,
                                    rxok: st.rxok,
                                    rwfw: st.rwfw,
                                    ackr: st.ackr,
                                    dwnb: st.dwnb,
                                    txnb: st.txnb,
                                },
                            ),
                        },
                    ),
                };
                let resp = serde_json::to_string(&resp)?;
                conn.publish(common_define::event::DeviceEvent::KAFKA_TOPIC, resp).await?;
                return Ok(());
            }
            lorawan_bridge::GatewayEventType::PushData(datas) => {
                for data in datas {
                    if let Ok(phy) = lora::parse::LoraMacDecode::switch(data.data.as_bytes()) {
                        let resp = match phy {
                            LoraPhy::Request(req) => common_define::event::DeviceEvent {
                                device: gateway_id,
                                event: common_define::event::DeviceEventType::Gateway(
                                    common_define::event::lora_gateway::GatewayEvent {
                                        eui: state.eui,
                                        time: Timestamp::from_timestamp_millis(
                                            state.time.timestamp_millis(),
                                        )
                                        .unwrap_or(Timestamp::now()),
                                        gateway_event: GatewayEventType::Join(
                                            common_define::event::lora_gateway::JoinPayload {
                                                app_eui: req.app_eui(),
                                                dev_eui: req.dev_eui(),
                                                dev_nonce: req.dev_nonce().to_string(),
                                            },
                                        ),
                                    },
                                ),
                            },
                            LoraPhy::Payload(payload) => common_define::event::DeviceEvent {
                                device: gateway_id,
                                event: common_define::event::DeviceEventType::Gateway(
                                    common_define::event::lora_gateway::GatewayEvent {
                                        eui: state.eui,
                                        time: Timestamp::from_timestamp_millis(
                                            state.time.timestamp_millis(),
                                        )
                                        .unwrap_or(Timestamp::now()),
                                        gateway_event: GatewayEventType::Data(
                                            common_define::event::lora_gateway::DataPayload {
                                                payload: data.data,
                                                f_port: payload.f_port().unwrap_or_default() as _,
                                                f_cnt: payload.fhdr().fcnt() as _,
                                                dev_addr: payload.dev_addr().to_string(),
                                                datr: data.datr,
                                                codr: data.codr.unwrap_or_default(),
                                                frequency: data.freq,
                                                rssi: data.rssi,
                                                snr: data.lsnr as _,
                                                channel: 0,
                                            },
                                        ),
                                    },
                                ),
                            },
                        };
                        let resp = serde_json::to_string(&resp)?;
                        conn.publish(common_define::event::DeviceEvent::KAFKA_TOPIC, resp).await?;
                    }
                }
            }
            lorawan_bridge::GatewayEventType::Pull => {}
            lorawan_bridge::GatewayEventType::TxAck => {}
        };

        Ok(())
    }
}
