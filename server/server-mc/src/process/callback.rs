use std::future::Future;

use anyhow::Result;

use crate::dao::server_config::ServerConfig;

pub trait ProcessService: Send + Sync + 'static {
    fn download_server_jar(
        &self,
        version: &str,
        to_path: &str,
    ) -> impl Future<Output = Result<()>> + Send;
    fn download_world(&self, uri: &str, to_path: &str) -> impl Future<Output = Result<()>> + Send;
    fn initialize_config_files(
        &self,
        root: &str,
        world_dir_name: &str,
        server_config: &ServerConfig,
    ) -> impl Future<Output = Result<()>> + Send;

    fn server_started(&self) -> impl Future<Output = ()> + Send;
    fn stdout_line(&self, line: &str) -> impl Future<Output = ()> + Send;
    fn server_stop(&self) -> impl Future<Output = ()> + Send;
}
