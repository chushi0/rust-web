use crate::dao::server_config::ServerConfig;

pub(super) enum Message {
    StartServerConfig(ServerConfig),
    StopServerConfig,
}
