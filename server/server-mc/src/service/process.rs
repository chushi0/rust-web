use std::{fs::File as StdFile, path::Path};

use anyhow::{anyhow, bail, Result};
use common::tonic_idl_gen::{
    StartServerConfigRequest, StartServerConfigResponse, StopServerConfigRequest,
    StopServerConfigResponse,
};
use futures_util::TryStreamExt;
use reqwest::Client;
use server_common::{
    db::context::{Context, ContextRef},
    external_api::aliyun::oss::OssClient,
};
use sqlx::{Database, MySql, Pool};
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::info;
use zip::ZipArchive;

use crate::{
    dao::{server_config::ServerConfigRepository, version::VersionRepository},
    process::manager::Manager,
};

pub struct ProcessService {
    db: Pool<MySql>,
    oss_client: OssClient,
    client: Client,
}

impl ProcessService {
    pub fn new(db: Pool<MySql>, oss_client: OssClient, client: Client) -> Self {
        Self {
            db,
            oss_client,
            client,
        }
    }
}

impl crate::process::callback::ProcessService for ProcessService {
    async fn download_server_jar(&self, version: &str, to_path: &str) -> anyhow::Result<()> {
        let mut db = Context::PoolRef(&self.db);
        let version = db
            .get_version_by_mcid(version)
            .await?
            .ok_or(anyhow!("version is not available"))?;

        let tmp_path = format!("{to_path}.download");
        let mut local_file = File::create(&tmp_path).await?;
        let mut bytes = self
            .client
            .get(version.server_url)
            .send()
            .await?
            .bytes_stream();

        while let Some(chunk) = bytes.try_next().await? {
            local_file.write_all(&chunk).await?;
        }

        tokio::fs::rename(tmp_path, to_path).await?;

        Ok(())
    }

    async fn download_world(&self, uri: &str, to_path: &str) -> anyhow::Result<()> {
        let tmp_path = format!("{to_path}.zip");
        let mut local_file = File::create(&tmp_path).await?;

        let client = self.oss_client.with_http(&self.client);
        let mut bytes = client.get_object(uri).await?;

        while let Some(chunk) = bytes.try_next().await? {
            local_file.write_all(&chunk).await?;
        }

        // extract world data
        let tmp_folder = format!("{to_path}.tmp");
        let final_folder = to_path.to_string();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let zip_file = StdFile::open(tmp_path)?;
            let mut archive = ZipArchive::new(zip_file)?;
            let target_path = Path::new(&tmp_folder);

            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;

                // 处理文件名编码（支持中文）
                let decoded_name = file
                    .enclosed_name()
                    .ok_or(anyhow!("invalid filename encode"))?;

                let normalized_name = target_path.join(decoded_name).canonicalize()?;
                if !normalized_name.starts_with(target_path) {
                    bail!("path crossing");
                }

                // 创建输出目录结构
                if file.is_dir() {
                    std::fs::create_dir_all(&normalized_name)?;
                } else {
                    // 确保父目录存在
                    if let Some(parent) = normalized_name.parent() {
                        std::fs::create_dir_all(parent)?;
                    }

                    // 写入文件内容
                    let mut out_file = StdFile::create(&normalized_name)?;
                    std::io::copy(&mut file, &mut out_file)?;
                }
            }

            std::fs::rename(tmp_folder, final_folder)?;
            Ok(())
        })
        .await??;

        Ok(())
    }

    async fn initialize_config_files(
        &self,
        root: &str,
        world_dir_name: &str,
    ) -> anyhow::Result<()> {
        // eula.txt
        {
            let path = Path::new(root).join("eula.txt");
            if !path.exists() {
                let mut file = File::create(path).await?;
                file.write_all(r#"eula=true"#.as_bytes()).await?;
            }
        }
        // server.properties
        {
            let public_server_host = "localhost"; // TODO

            let path = Path::new(root).join("server.properties");
            let mut file = File::create(path).await?;
            file.write_all("difficulty=hard\n".as_bytes()).await?;
            file.write_all("view-distance=16\n".as_bytes()).await?;
            file.write_all(format!("level-name={world_dir_name}\n").as_bytes())
                .await?;
            file.write_all("enable-command-block=true\n".as_bytes())
                .await?;
            file.write_all(
                format!("resource-pack=http://{public_server_host}/api/mc-resource-pack?id=\n")
                    .as_bytes(),
            )
            .await?;
        }
        Ok(())
    }

    async fn server_started(&self) -> () {
        info!("server started");
    }

    async fn server_stop(&self) -> () {
        info!("server stopped");
    }

    async fn stdout_line(&self, line: &str) -> () {
        info!("stdout: {}", line);
    }
}

pub async fn start_server_config<DB: Database>(
    db: ContextRef<'_, '_, DB>,
    manager: &Manager,
    req: StartServerConfigRequest,
) -> Result<StartServerConfigResponse>
where
    for<'db> Context<'db, DB>: ServerConfigRepository,
{
    let Some(server_config) = db.get_server_config_by_id(req.id).await? else {
        bail!("server config not found");
    };

    manager.start_server_config(server_config).await?;

    Ok(StartServerConfigResponse {})
}

pub async fn stop_server_config(
    manager: &Manager,
    _req: StopServerConfigRequest,
) -> Result<StopServerConfigResponse> {
    manager.stop_server_config().await?;
    Ok(StopServerConfigResponse {})
}
