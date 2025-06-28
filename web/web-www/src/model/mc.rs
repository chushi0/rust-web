use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ListServerConfigRequest {
    pub offset: u64,
    pub limit: u64,
}

#[derive(Debug, Deserialize)]
pub struct ListServerConfigResponse {
    pub count: i64,
    pub configs: Vec<ServerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub id: u64,
    pub name: String,
    pub version: String,
    pub motd: String,
}

#[derive(Debug, Serialize)]
pub struct ListMcVersionRequest {
    pub offset: u64,
    pub limit: u64,
    pub has_snapshot: bool,
}

#[derive(Debug, Deserialize)]
pub struct ListMcVersionResponse {
    #[allow(unused)]
    pub count: i64,
    pub versions: Vec<McVersion>,
}

#[derive(Debug, Deserialize)]
pub struct McVersion {
    pub id: String,
    #[allow(unused)]
    pub snapshot: bool,
}

#[derive(Debug, Serialize)]
pub struct CreateServerConfigRequest {
    pub name: String,
    pub version: String,
    pub world_uri: Option<String>,
    pub resource_uri: Option<String>,
    pub motd: String,
}

#[derive(Debug, Serialize)]
pub struct StartServerConfigRequest {
    pub id: u64,
}

#[derive(Debug, Deserialize)]
pub struct GetCurrentServerConfigResponse {
    #[allow(unused)]
    pub running_config: Option<ServerConfig>,
    pub status: HashMap<RunningServerStage, RunningServerStageInfo>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

#[derive(Debug, Deserialize)]
pub struct RunningServerStageInfo {
    pub enter_time: i64,
    pub finish_time: Option<i64>,
    pub in_error: bool,
    #[allow(unused)]
    pub error_message: Option<String>,
}
