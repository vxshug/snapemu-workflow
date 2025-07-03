use chrono::Utc;
use json_define::data::{LoraDataDown, LoraDataDownAck};
use json_define::to_json_string;
use snap_mq::output::SnapResult;
use snap_mq::publish::kafka::KafkaPub;
use snap_mq::publish::PublishMessage;
use snap_mq::timeout::TimeoutTasks;
use snap_mq::SnapError;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct SendManager {
    list: TimeoutTasks<Uuid>,
    send: KafkaPub,
}

impl SendManager {
    pub(crate) fn new(send: KafkaPub) -> Self {
        Self {
            list: TimeoutTasks::new(),
            send,
        }
    }

    pub(crate) async fn send<M, T>(
        &self,
        id: Uuid,
        msg: M,
        timeout: T,
    ) -> SnapResult<LoraDataDownAck>
    where
        M: Into<String>,
        T: Into<Option<i64>>,
    {
        let time = timeout.into().unwrap_or(10);
        if time < 1 {
            return Err(SnapError::warn("timeout"));
        }
        let msg = msg.into();
        let msg_id = Uuid::new_v4();
        let data = LoraDataDown {
            node_id: id,
            msg_id,
            data: msg,
            time: Utc::now(),
        };
        self.send
            .publish_message((LoraDataDown::topic(), "", to_json_string(&data)?))
            .await?;

        let timeout = self.list.timeout(msg_id, time as u64);
        timeout.wait().await
    }
}
