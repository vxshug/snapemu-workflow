use serde::{Deserialize, Serialize};
use derive_new::new;


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, new)]
pub struct DataWrapper<T> {
    pub id: u32,
    pub data: T,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "msg_type")]
pub enum GwCmd {
    ShellCmd(DataWrapper<ShellCmd>),
    Config(DataWrapper<()>),
    UpdateConfig(DataWrapper<snap_proto::manager::GwConfig>)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "msg_type")]
pub enum GwCmdResponse {
    ShellCmd(DataWrapper<ShellCmdResponse>),
    Config(DataWrapper<snap_proto::manager::GwConfig>),
    UpdateConfig(DataWrapper<snap_proto::manager::GwConfig>)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, new)]
pub struct ShellCmd {
    #[serde(rename = "cmd_str")]
    pub cmd_str: String,
    #[serde(rename = "cmd_timeout_ms")]
    pub cmd_timeout_ms: i32,
}


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, new)]
pub struct ShellCmdResponse {
    #[serde(rename = "exc_result")]
    pub exc_result: String,
    #[serde(rename = "exc_result_code")]
    pub exc_result_code: i32,
}

