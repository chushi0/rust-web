use std::{
    collections::HashMap,
    fmt::Display,
    path::Path,
    sync::{Arc, OnceLock},
};

use anyhow::{bail, Result};
use chrono::Utc;
use regex::Regex;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    RwLock,
};
use tracing::{info, warn};

use crate::dao::server_config::ServerConfig;

use super::{
    callback::ProcessService,
    communicate::Message,
    lifecycle::ProcessLifeCycle,
    status::{ProcessStatus, StartingStatus, StatusInfo},
    RUN_DIR, SERVER_JAR_DIR,
};

#[derive(Clone)]
pub struct Manager {
    inner: Arc<ManagerInner>,
}

struct ManagerInner {
    message_sender: Sender<Message>,
    status: RwLock<HashMap<ProcessStatus, StatusInfo>>,
}

impl Manager {
    pub fn new(service: impl ProcessService) -> Manager {
        let inner = ManagerInner::new(service);

        Manager { inner }
    }

    pub async fn start_server_config(&self, server_config: ServerConfig) -> Result<()> {
        self.inner
            .message_sender
            .send(Message::StartServerConfig(server_config))
            .await?;

        Ok(())
    }

    pub async fn stop_server_config(&self) -> Result<()> {
        self.inner
            .message_sender
            .send(Message::StopServerConfig)
            .await?;

        Ok(())
    }
}

impl ManagerInner {
    fn new(service: impl ProcessService) -> Arc<ManagerInner> {
        let (sender, receiver) = mpsc::channel(10);
        let manager = Arc::new(ManagerInner {
            message_sender: sender,
            status: RwLock::new(HashMap::new()),
        });
        tokio::spawn(manager_loop(service, manager.clone(), receiver));
        manager
    }

    async fn clean_status(&self) {
        self.status.write().await.clear();
    }

    async fn start_status(&self, status: ProcessStatus) {
        info!("enter status: {:?}", status);
        let mut map = self.status.write().await;
        map.iter_mut()
            .for_each(|(_, value)| value.end_time = value.end_time.or_else(|| Some(Utc::now())));
        map.insert(
            status,
            StatusInfo {
                start_time: Utc::now(),
                ..Default::default()
            },
        );
    }

    async fn status_error(&self, error: impl Display) {
        warn!("status error: {}", error);
        self.status.write().await.iter_mut().for_each(|(_, value)| {
            if value.end_time.is_none() {
                value.end_time = Some(Utc::now());
                value.error = Some(error.to_string());
            }
        });
    }
}

async fn manager_loop(
    service: impl ProcessService,
    manager: Arc<ManagerInner>,
    mut receiver: Receiver<Message>,
) {
    loop {
        let Some(message) = receiver.recv().await else {
            break;
        };

        let Message::StartServerConfig(server_config) = message else {
            continue;
        };

        manager.clean_status().await;

        let mut process_lifecycle = match starting_service(&service, &manager, &server_config).await
        {
            Ok(process_lifecycle) => process_lifecycle,
            Err(e) => {
                manager.status_error(e).await;
                continue;
            }
        };

        service.server_started().await;
        manager.start_status(ProcessStatus::Running).await;

        loop {
            tokio::select! {
                message = receiver.recv() => {
                    let Some(message) = message else {
                        break;
                    };

                    match message {
                        Message::StartServerConfig(_server_config) => continue,
                        Message::StopServerConfig => break,
                    }
                },
                line = process_lifecycle.read_line() => {
                    match line {
                        Ok(Some(line)) => {
                            service.stdout_line(&line).await;
                        },
                        Ok(None) | Err(_) => {
                            break;
                        }
                    };
                }
            };
        }

        info!("stoping service");
        manager.start_status(ProcessStatus::Terminating).await;
        if let Err(e) = process_lifecycle.stop_service().await {
            manager.status_error(e).await;
        }

        service.server_stop().await;

        manager.start_status(ProcessStatus::Terminated).await;
    }
}

async fn starting_service(
    service: &impl ProcessService,
    manager: &ManagerInner,
    server_config: &ServerConfig,
) -> Result<ProcessLifeCycle> {
    let jar_path = format!("{}/{}.jar", SERVER_JAR_DIR, &server_config.mc_version);
    let world_path = format!("{}/{}", RUN_DIR, server_config.id);

    if let Some(parent) = Path::new(&jar_path).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    if !Path::new(RUN_DIR).exists() {
        std::fs::create_dir_all(Path::new(RUN_DIR))?;
    }
    info!("starting service, jar_path: {jar_path}, world_path: {world_path}");

    // download jar
    manager
        .start_status(ProcessStatus::Starting(StartingStatus::DownloadServerJar))
        .await;
    if !Path::new(&jar_path).exists() {
        service
            .download_server_jar(&server_config.mc_version, &jar_path)
            .await?;
    }

    // download world
    manager
        .start_status(ProcessStatus::Starting(StartingStatus::DownloadWorld))
        .await;
    if !Path::new(&world_path).exists() {
        if let Some(world_uri) = &server_config.world_uri {
            service.download_world(&world_uri, &world_path).await?;
        }
    }

    // initialize configuration
    manager
        .start_status(ProcessStatus::Starting(
            StartingStatus::InitializeConfigFile,
        ))
        .await;
    service
        .initialize_config_files(RUN_DIR, &server_config.id.to_string())
        .await?;

    // starting server and wait to ready
    manager
        .start_status(ProcessStatus::Starting(
            StartingStatus::WaitingForServerReady,
        ))
        .await;
    let mut lifecycle = ProcessLifeCycle::start(&jar_path)?;
    // loop to detect 'Done' message
    while let Some(message) = lifecycle.read_line().await? {
        const DONE_PATTERN: &str = r#"\[(?:\d{2}:?){3}\] \[Server thread/INFO\]: Done \((?:\d+\.\d+)s\)! For help, type \"help\""#;
        static DONE_REGEX: OnceLock<Regex> = OnceLock::new();

        let done_regex = DONE_REGEX
            .get_or_init(|| Regex::new(DONE_PATTERN).expect("done regex is not available"));
        if done_regex.is_match(&message) {
            return Ok(lifecycle);
        }
    }

    bail!("process shutdown before getting ready")
}
