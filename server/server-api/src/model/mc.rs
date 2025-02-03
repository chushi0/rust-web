use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ListMcVersionRequest {
    #[serde(default = "super::default_offset")]
    pub offset: u64,
    #[serde(default = "super::default_limit")]
    pub limit: u64,
    #[serde(default = "super::default_false")]
    pub has_snapshot: bool,
}

#[derive(Debug, Serialize)]
pub struct ListMcVersionResponse {
    pub count: i64,
    pub versions: Vec<McVersion>,
}

#[derive(Debug, Serialize)]
pub struct McVersion {
    pub id: String,
    pub snapshot: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateServerConfigRequest {
    pub name: String,
    pub version: String,
    pub world_uri: Option<String>,
    pub resource_uri: Option<String>,
    pub motd: String,
}

#[derive(Debug, Deserialize)]
pub struct ListServerConfigRequest {
    #[serde(default = "super::default_offset")]
    pub offset: u64,
    #[serde(default = "super::default_limit")]
    pub limit: u64,
}

#[derive(Debug, Serialize)]
pub struct ListServerConfigResponse {
    pub count: i64,
    pub configs: Vec<ServerConfig>,
}

#[derive(Debug, Serialize)]
pub struct ServerConfig {
    pub id: u64,
    pub name: String,
    pub version: String,
    pub motd: String,
}

#[derive(Debug, Deserialize)]
pub struct StartServerConfigRequest {
    pub id: u64,
}

#[derive(Debug, Serialize)]
pub struct GetCurrentServerConfigResponse {
    pub running_config: Option<ServerConfig>,
    pub status: HashMap<RunningServerStage, RunningServerStageInfo>,
}

#[derive(Debug, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RunningServerStage {
    Init,
    PullingServer,
    PullingWorld,
    InitializingFile,
    Starting,
    Running,
    Stopping,
    Stopped,
}

#[derive(Debug, Serialize)]
pub struct RunningServerStageInfo {
    pub enter_time: i64,
    pub finish_time: Option<i64>,
    pub in_error: bool,
    pub error_message: Option<String>,
}
