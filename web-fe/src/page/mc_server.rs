use std::{cell::RefCell, collections::HashMap, rc::Rc};

use anyhow::{anyhow, Error, Result};
use gloo_net::http::Headers;
use gloo_timers::callback::Interval;
use log::{error, info};
use wasm_bindgen::JsCast;
use web_sys::{File, HtmlInputElement, HtmlTextAreaElement};
use yew::{html::Scope, prelude::*};

use crate::{
    component::*,
    model::{
        mc::{
            CreateServerConfigRequest, GetCurrentServerConfigResponse, ListMcVersionRequest,
            ListMcVersionResponse, ListServerConfigRequest, ListServerConfigResponse, McVersion,
            RunningServerStage, RunningServerStageInfo, ServerConfig, StartServerConfigRequest,
        },
        oss::GetUploadSignatureResponse,
        EncryptRequest,
    },
    sys::bootstrap::modal::Modal,
};

pub struct McServerManagePage {
    // interval
    // 保留此信息，在drop后interval会停止
    _fetch_current_server_config_interval: Interval,

    // node
    create_server_config_dialog: NodeRef,
    create_server_config_modal: Rc<RefCell<Option<Modal>>>,
    create_server_config_name: NodeRef,
    create_server_config_version: NodeRef,
    create_server_config_motd: NodeRef,

    // data
    current_server_config: HashMap<RunningServerStage, RunningServerStageInfo>,
    current_server_config_status: RunningServerStage,
    current_server_config_error: bool,
    server_config_total: Option<i64>,
    server_configs: Option<Vec<ServerConfig>>,
    server_config_page: u64,
    mc_versions: Vec<String>,
    with_snapshot_version: bool,

    // status
    upload_world: UploadUriStatus,
    upload_resource: UploadUriStatus,
    submit_status: SubmitStatus,
}

#[derive(Debug, Default)]
enum UploadUriStatus {
    #[default]
    NoUpload,
    Uploading,
    Finish {
        uri: String,
    },
    Error {
        err: String,
    },
}

impl UploadUriStatus {
    fn is_can_submit(&self) -> bool {
        matches!(self, Self::NoUpload | Self::Finish { .. })
    }
}

#[derive(Debug, Default)]
enum SubmitStatus {
    #[default]
    Idle,
    Submitting,
    Error {
        err: String,
    },
}

pub enum McServerManagePageMsg {
    LoadCurrentServerConfig {
        current_server_config: HashMap<RunningServerStage, RunningServerStageInfo>,
    },
    LoadServerConfig {
        total: i64,
        server_configs: Vec<ServerConfig>,
    },
    LoadMcVersions {
        versions: Vec<McVersion>,
    },
    SwitchServerConfigPage {
        page: u64,
    },
    ChangeVersionWithSnapshot {
        with_snapshot: bool,
    },
    UploadWorldFile {
        file: File,
    },
    UploadResourceFile {
        file: File,
    },
    FinishUploadWorldFile {
        uri: String,
    },
    FinishUploadResourceFile {
        uri: String,
    },
    ErrorUploadWorldFile {
        err: String,
    },
    ErrorUploadResourceFile {
        err: String,
    },
    SubmitCreateServerConfig,
    FinishCreateServerConfig,
    ErrorCreateServerConfig {
        err: String,
    },
}

impl McServerManagePage {
    fn load_server_configs(page: u64, link: Scope<Self>) {
        wasm_bindgen_futures::spawn_local(async move {
            match Self::load_server_configs_imp(ListServerConfigRequest {
                offset: (page - 1) * 10,
                limit: 10,
            })
            .await
            {
                Ok(config) => {
                    link.send_message(McServerManagePageMsg::LoadServerConfig {
                        total: config.count,
                        server_configs: config.configs,
                    });
                }
                Err(err) => error!("{}", err),
            }
        });
    }

    async fn load_server_configs_imp(
        req: ListServerConfigRequest,
    ) -> Result<ListServerConfigResponse> {
        let encrypt = EncryptRequest::encrypt_payload(&serde_json::to_vec(&req)?)?;

        Ok(gloo_net::http::Request::get("/api/mc/server_config/list")
            .query(encrypt.to_query_params())
            .send()
            .await?
            .json()
            .await?)
    }

    fn upload_file<OnFinish, OnError>(
        file: File,
        on_finish: OnFinish,
        on_error: OnError,
        link: Scope<Self>,
    ) where
        OnFinish: FnOnce(String) -> McServerManagePageMsg + Send + 'static,
        OnError: FnOnce(Error) -> McServerManagePageMsg + Send + 'static,
    {
        wasm_bindgen_futures::spawn_local(async move {
            let msg = match Self::upload_file_imp(file).await {
                Ok(uri) => on_finish(uri),
                Err(err) => on_error(err),
            };
            link.send_message(msg);
        });
    }

    async fn upload_file_imp(file: File) -> Result<String> {
        // 1. get uri from server
        let signature: GetUploadSignatureResponse =
            gloo_net::http::Request::get("/api/oss/upload?content_type=application/zip")
                .send()
                .await?
                .json()
                .await?;

        // 2. upload to oss
        let headers = Headers::new();
        headers.append("authorization", &signature.signature);
        headers.append("content-type", "application/zip");
        signature.extra_headers.iter().for_each(|header| {
            headers.append(&header.key, &header.value);
        });
        let upload_oss_result = gloo_net::http::Request::put(&signature.url)
            .headers(headers)
            .body(gloo_file::Blob::from(file))
            .expect("failed to upload file")
            .send()
            .await?;

        if upload_oss_result.status() != 200 {
            return Err(anyhow!(
                "upload failed with status: {}",
                upload_oss_result.status()
            ));
        }

        Ok(signature.uri)
    }

    fn load_mc_versions(with_snapshot: bool, link: Scope<Self>) {
        wasm_bindgen_futures::spawn_local(async move {
            match Self::load_mc_versions_imp(with_snapshot).await {
                Ok(versions) => {
                    link.send_message(McServerManagePageMsg::LoadMcVersions { versions });
                }
                Err(err) => error!("{}", err),
            }
        });
    }

    async fn load_mc_versions_imp(with_snapshot: bool) -> Result<Vec<McVersion>> {
        let encrypt =
            EncryptRequest::encrypt_payload(&serde_json::to_vec(&ListMcVersionRequest {
                offset: 0,
                limit: 100,
                has_snapshot: with_snapshot,
            })?)?;

        let list_versions: ListMcVersionResponse =
            gloo_net::http::Request::get("/api/mc/version/list")
                .query(encrypt.to_query_params())
                .send()
                .await?
                .json()
                .await?;

        Ok(list_versions.versions)
    }

    fn create_server_config(req: CreateServerConfigRequest, link: Scope<Self>) {
        wasm_bindgen_futures::spawn_local(async move {
            match Self::create_server_config_imp(req).await {
                Ok(()) => link.send_message(McServerManagePageMsg::FinishCreateServerConfig),
                Err(err) => link.send_message(McServerManagePageMsg::ErrorCreateServerConfig {
                    err: err.to_string(),
                }),
            }
        });
    }

    async fn create_server_config_imp(req: CreateServerConfigRequest) -> Result<()> {
        let encrypt = EncryptRequest::encrypt_payload(&serde_json::to_vec(&req)?)?;
        let data = rmp_serde::to_vec(&encrypt)?;

        gloo_net::http::Request::post("/api/mc/server_config/create")
            .body(data)?
            .send()
            .await?;

        Ok(())
    }

    fn load_current_server_config(link: Scope<Self>) {
        wasm_bindgen_futures::spawn_local(async move {
            match Self::load_current_server_config_imp().await {
                Ok(current_server_config) => {
                    link.send_message(McServerManagePageMsg::LoadCurrentServerConfig {
                        current_server_config,
                    });
                }
                Err(err) => error!("{err}"),
            };
        });
    }

    async fn load_current_server_config_imp(
    ) -> Result<HashMap<RunningServerStage, RunningServerStageInfo>> {
        let encrypt = EncryptRequest::encrypt_payload(&serde_json::to_vec(&())?)?;

        let response: GetCurrentServerConfigResponse =
            gloo_net::http::Request::get("/api/mc/server_config/process/info")
                .query(encrypt.to_query_params())
                .send()
                .await?
                .json()
                .await?;

        Ok(response.status)
    }

    fn start_server_config(id: u64) {
        wasm_bindgen_futures::spawn_local(async move {
            match Self::start_server_config_imp(id).await {
                Ok(_) => (),
                Err(e) => error!("{e}"),
            };
        });
    }

    async fn start_server_config_imp(id: u64) -> Result<()> {
        let encrypt =
            EncryptRequest::encrypt_payload(&serde_json::to_vec(&StartServerConfigRequest {
                id,
            })?)?;
        let data = rmp_serde::to_vec(&encrypt)?;

        gloo_net::http::Request::post("/api/mc/server_config/process/start")
            .body(data)?
            .send()
            .await?;

        Ok(())
    }

    fn stop_server_config() {
        wasm_bindgen_futures::spawn_local(async move {
            match Self::stop_server_config_imp().await {
                Ok(_) => (),
                Err(e) => error!("{e}"),
            };
        });
    }

    async fn stop_server_config_imp() -> Result<()> {
        let encrypt = EncryptRequest::encrypt_payload(&serde_json::to_vec(&())?)?;
        let data = rmp_serde::to_vec(&encrypt)?;

        gloo_net::http::Request::post("/api/mc/server_config/process/stop")
            .body(data)?
            .send()
            .await?;

        Ok(())
    }
}

impl Component for McServerManagePage {
    type Message = McServerManagePageMsg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        Self::load_server_configs(1, ctx.link().clone());
        Self::load_mc_versions(false, ctx.link().clone());
        Self::load_current_server_config(ctx.link().clone());

        let fetch_current_server_config_interval = {
            let link = ctx.link().clone();
            Interval::new(5000, move || {
                Self::load_current_server_config(link.clone());
            })
        };

        Self {
            _fetch_current_server_config_interval: fetch_current_server_config_interval,
            current_server_config: HashMap::new(),
            current_server_config_status: RunningServerStage::Init,
            current_server_config_error: false,
            create_server_config_dialog: NodeRef::default(),
            create_server_config_modal: Rc::new(RefCell::new(None)),
            create_server_config_name: NodeRef::default(),
            create_server_config_version: NodeRef::default(),
            create_server_config_motd: NodeRef::default(),
            server_config_page: 1,
            server_configs: None,
            server_config_total: None,
            mc_versions: Vec::new(),
            with_snapshot_version: false,
            upload_world: UploadUriStatus::default(),
            upload_resource: UploadUriStatus::default(),
            submit_status: SubmitStatus::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            McServerManagePageMsg::LoadCurrentServerConfig {
                current_server_config,
            } => {
                self.current_server_config = current_server_config;
                self.current_server_config_status = self
                    .current_server_config
                    .keys()
                    .max()
                    .copied()
                    .unwrap_or(RunningServerStage::Init);
                self.current_server_config_error = self
                    .current_server_config
                    .values()
                    .any(|info| info.in_error);
                return true;
            }
            McServerManagePageMsg::LoadServerConfig {
                total,
                server_configs,
            } => {
                self.server_config_total = Some(total);
                self.server_configs = Some(server_configs);
                return true;
            }
            McServerManagePageMsg::LoadMcVersions { versions } => {
                self.mc_versions = versions.into_iter().map(|version| version.id).collect();
                return true;
            }
            McServerManagePageMsg::SwitchServerConfigPage { page } => {
                self.server_configs = None;
                self.server_config_page = page;
                Self::load_server_configs(page, ctx.link().clone());
                return true;
            }
            McServerManagePageMsg::ChangeVersionWithSnapshot { with_snapshot } => {
                self.with_snapshot_version = with_snapshot;
                Self::load_mc_versions(with_snapshot, ctx.link().clone());
                return false;
            }
            McServerManagePageMsg::UploadWorldFile { file } => {
                self.upload_world = UploadUriStatus::Uploading;
                Self::upload_file(
                    file,
                    |uri| McServerManagePageMsg::FinishUploadWorldFile { uri },
                    |err| McServerManagePageMsg::ErrorUploadWorldFile {
                        err: err.to_string(),
                    },
                    ctx.link().clone(),
                );
                return true;
            }
            McServerManagePageMsg::UploadResourceFile { file } => {
                self.upload_resource = UploadUriStatus::Uploading;
                Self::upload_file(
                    file,
                    |uri| McServerManagePageMsg::FinishUploadResourceFile { uri },
                    |err| McServerManagePageMsg::ErrorUploadResourceFile {
                        err: err.to_string(),
                    },
                    ctx.link().clone(),
                );
                return true;
            }
            McServerManagePageMsg::FinishUploadWorldFile { uri } => {
                self.upload_world = UploadUriStatus::Finish { uri };
                return true;
            }
            McServerManagePageMsg::FinishUploadResourceFile { uri } => {
                self.upload_resource = UploadUriStatus::Finish { uri };
                return true;
            }
            McServerManagePageMsg::ErrorUploadWorldFile { err } => {
                self.upload_world = UploadUriStatus::Error { err };
                return true;
            }
            McServerManagePageMsg::ErrorUploadResourceFile { err } => {
                self.upload_resource = UploadUriStatus::Error { err };
                return true;
            }
            McServerManagePageMsg::SubmitCreateServerConfig => {
                self.submit_status = SubmitStatus::Submitting;

                let name = self
                    .create_server_config_name
                    .cast::<HtmlInputElement>()
                    .expect("name should be a input element")
                    .value();

                let version = self
                    .create_server_config_version
                    .cast::<HtmlInputElement>()
                    .expect("version should be a input element")
                    .value();

                let world_uri = if let UploadUriStatus::Finish { uri } = &self.upload_world {
                    Some(uri.clone())
                } else {
                    None
                };

                let resource_uri = if let UploadUriStatus::Finish { uri } = &self.upload_resource {
                    Some(uri.clone())
                } else {
                    None
                };

                let motd = self
                    .create_server_config_motd
                    .cast::<HtmlTextAreaElement>()
                    .expect("motd should be a text area element")
                    .value();

                info!("name: {name}, version: {version}, world_uri: {world_uri:?}, resource_uri: {resource_uri:?}, motd: {motd}");

                Self::create_server_config(
                    CreateServerConfigRequest {
                        name,
                        version,
                        world_uri,
                        resource_uri,
                        motd,
                    },
                    ctx.link().clone(),
                );

                return true;
            }
            McServerManagePageMsg::FinishCreateServerConfig => {
                self.submit_status = SubmitStatus::Idle;

                if let Some(modal) = self.create_server_config_modal.borrow().as_ref() {
                    modal.hide();
                }

                Self::load_server_configs(self.server_config_page, ctx.link().clone());
                return true;
            }
            McServerManagePageMsg::ErrorCreateServerConfig { err } => {
                self.submit_status = SubmitStatus::Error { err };
                return true;
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let show_create_server_config_dialog = {
            let dialog = self.create_server_config_dialog.clone();
            let modal = self.create_server_config_modal.clone();
            Callback::from(move |_e: MouseEvent| {
                if modal.borrow().is_none() {
                    let element = dialog
                        .cast()
                        .expect("Cannot cast dialog node_ref to Element");
                    *modal.borrow_mut() = Some(Modal::new_with_element(&element))
                }
                modal
                    .borrow()
                    .as_ref()
                    .expect("modal should be initialized before")
                    .show();
            })
        };

        let world_file_select = {
            let link = ctx.link().clone();
            Callback::from(move |e: Event| {
                let input = e
                    .target()
                    .expect("event target should be exists")
                    .dyn_into::<HtmlInputElement>()
                    .expect("this noderef should be input element");
                let Some(file) = input.files().and_then(|files| files.get(0)) else {
                    return;
                };

                link.send_message(McServerManagePageMsg::UploadWorldFile { file });
            })
        };

        let resource_file_select = {
            let link = ctx.link().clone();
            Callback::from(move |e: Event| {
                let input = e
                    .target()
                    .expect("event target should be exists")
                    .dyn_into::<HtmlInputElement>()
                    .expect("this noderef should be input element");
                let Some(file) = input.files().and_then(|files| files.get(0)) else {
                    return;
                };
                link.send_message(McServerManagePageMsg::UploadResourceFile { file });
            })
        };

        let on_change_with_snapshot = {
            let link = ctx.link().clone();
            Callback::from(move |e: Event| {
                let with_snapshot = e
                    .target()
                    .expect("event target should be exists")
                    .dyn_into::<HtmlInputElement>()
                    .expect("this noderef should be input element")
                    .checked();
                link.send_message(McServerManagePageMsg::ChangeVersionWithSnapshot {
                    with_snapshot,
                });
            })
        };

        let on_submit_form = {
            let link = ctx.link().clone();
            Callback::from(move |e: SubmitEvent| {
                e.prevent_default();
                link.send_message(McServerManagePageMsg::SubmitCreateServerConfig);
            })
        };

        let on_start_config = Callback::from(|id: u64| {
            Self::start_server_config(id);
        });

        let on_stop_config = Callback::from(|_e: MouseEvent| {
            Self::stop_server_config();
        });

        let server_config_prev_page = {
            let page = self.server_config_page;
            let link = ctx.link().clone();
            Callback::from(move |_e: MouseEvent| {
                if page > 1 {
                    link.send_message(McServerManagePageMsg::SwitchServerConfigPage {
                        page: page - 1,
                    });
                }
            })
        };

        let server_config_next_page = {
            let page = self.server_config_page;
            let total = self.server_config_total;
            let link = ctx.link().clone();
            Callback::from(move |_e: MouseEvent| {
                if page * 10 < (total.unwrap_or(0) as u64) {
                    link.send_message(McServerManagePageMsg::SwitchServerConfigPage {
                        page: page + 1,
                    });
                }
            })
        };

        let max_page = {
            let total = self.server_config_total.unwrap_or(0);
            if total % 10 == 0 {
                (total / 10) as u64
            } else {
                (total / 10 + 1) as u64
            }
            .max(1)
        };

        let server_config_first_page = {
            let link = ctx.link().clone();
            Callback::from(move |_e: MouseEvent| {
                link.send_message(McServerManagePageMsg::SwitchServerConfigPage { page: 1 });
            })
        };

        let server_config_last_page = {
            let link = ctx.link().clone();
            Callback::from(move |_e: MouseEvent| {
                link.send_message(McServerManagePageMsg::SwitchServerConfigPage { page: max_page });
            })
        };

        html! {
            <>
                <NavBar active="manage-mc-server" />
                <Title title={"MC服务器 - 管理"} />
                <div class="container-sm">
                    <h3>{"MC 服务器管理"}</h3>

                    <div class="card">
                        <div class="card-header">
                            <span class="card-text">
                                {"当前状态："}
                                {
                                    if self.current_server_config_error {
                                        html! {
                                            <span class="badge bg-danger">{"已停止"}</span>
                                        }
                                    } else {
                                        match self.current_server_config_status {
                                            RunningServerStage::Init | RunningServerStage::Stopped => html! {
                                                <span class="badge bg-info text-dark">{"已停止"}</span>
                                            },
                                            RunningServerStage::PullingServer | RunningServerStage::PullingWorld | RunningServerStage::InitializingFile | RunningServerStage::Starting => html! {
                                                <span class="badge bg-primary">{"启动中"}</span>
                                            },
                                            RunningServerStage::Running => html! {
                                                <span class="badge bg-success">{"运行中"}</span>
                                            },
                                            RunningServerStage::Stopping => html! {
                                                <span class="badge bg-info text-dark">{"停止中"}</span>
                                            }
                                        }
                                    }
                                }

                            </span>
                            <button type="button" class="btn btn-sm btn-outline-danger float-end" onclick={on_stop_config}>{"停止服务器"}</button>
                        </div>
                        <div class="card-body" style="overflow-x: auto; padding: 2rem;">
                            <div class="row g-0" style="width: 2000px;">
                                {
                                    [
                                        (RunningServerStage::PullingServer, "下载服务端程序", true),
                                        (RunningServerStage::PullingWorld, "下载存档", true),
                                        (RunningServerStage::InitializingFile, "生成配置文件", true),
                                        (RunningServerStage::Starting, "启动服务器", true),
                                        (RunningServerStage::Running, "等待游戏结束", true),
                                        (RunningServerStage::Stopping, "停止服务器", false)
                                    ].into_iter().map(|(stage, title, has_next)| {
                                        let stage = self.current_server_config.get(&stage);
                                        let icon = if let Some(stage) = stage {
                                            if stage.in_error {
                                                ProgressIcon::Error
                                            } else if stage.finish_time.is_some() {
                                                ProgressIcon::Pass
                                            } else {
                                                ProgressIcon::Running
                                            }
                                        } else {
                                            ProgressIcon::Waiting
                                        };

                                        let start_time = stage.as_ref().map(|stage| stage.enter_time);
                                        let end_time = stage.as_ref().and_then(|stage| stage.finish_time);

                                        html! {
                                            <ProgressNode title={title} has_next={has_next} icon={icon} start_time={start_time} end_time={end_time} />
                                        }
                                    }).collect::<Html>()
                                }
                            </div>
                        </div>
                    </div>

                    <h4 style="margin-top: 24px;">{"已创建的服务器"}</h4>

                    <button type="button" class="btn btn-outline-primary float-end" style="display: inline-block;" onclick={show_create_server_config_dialog}>{"新增"}</button>

                    <table class="table table-hover">
                        <thead>
                            <tr>
                                <th scope="col">{"#"}</th>
                                <th scope="col">{"服务器名"}</th>
                                <th scope="col">{"MC版本"}</th>
                                <th scope="col">{"MOTD"}</th>
                                <th scope="col">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                if let Some(server_configs) = &self.server_configs {
                                    server_configs.iter().map(|server_config| html!{
                                        <ServerConfigNode
                                            id={server_config.id}
                                            name={server_config.name.clone()}
                                            version={server_config.version.clone()}
                                            motd={server_config.motd.clone()}
                                            onstart={on_start_config.clone()} />
                                    }).collect::<Html>()
                                } else {
                                    (0..3).into_iter().map(|_| html!{
                                        <ServerConfigPlaceHolder />
                                    }).collect::<Html>()
                                }
                            }
                        </tbody>
                    </table>

                    {
                        if max_page > 1 {
                            html!{
                                <nav>
                                    <ul class="pagination justify-content-center">
                                        {
                                            if self.server_config_page > 1 {
                                                html!{ <li class="page-item"><a class="page-link" href="javascript::" onclick={server_config_prev_page}>{"上一页"}</a></li> }
                                            } else {
                                                html!{ <li class="page-item disabled"><a class="page-link">{"上一页"}</a></li> }
                                            }
                                        }
                                        {
                                            if self.server_config_page > 1 {
                                                html!{ <li class="page-item"><a class="page-link" href="javascript::" onclick={server_config_first_page}>{1}</a></li> }
                                            } else {
                                                html!{}
                                            }
                                        }
                                        {
                                            if self.server_config_page > 2 {
                                                html!{ <li class="page-item disabled"><a class="page-link">{"..."}</a></li> }
                                            }
                                            else {
                                                html! {}
                                            }
                                        }
                                        <li class="page-item disabled"><a class="page-link" aria-current="page">{self.server_config_page}</a></li>
                                        {
                                            if self.server_config_page < max_page - 1 {
                                                html!{ <li class="page-item disabled"><a class="page-link">{"..."}</a></li> }
                                            }
                                            else {
                                                html! {}
                                            }
                                        }
                                        {
                                            if self.server_config_page < max_page {
                                                html!{ <li class="page-item"><a class="page-link" href="javascript::" onclick={server_config_last_page}>{max_page}</a></li> }
                                            }
                                            else {
                                                html! {}
                                            }
                                        }
                                        {
                                            if self.server_config_page < max_page {
                                                html!{ <li class="page-item"><a class="page-link" href="javascript::" onclick={server_config_next_page}>{"下一页"}</a></li> }
                                            }
                                            else {
                                                html! { <li class="page-item disabled"><a class="page-link">{"下一页"}</a></li> }
                                            }
                                        }
                                    </ul>
                                </nav>
                            }
                        } else {
                            html!{}
                        }
                    }

                    <div class="modal fade" ref={self.create_server_config_dialog.clone()}>
                        <div class="modal-dialog modal-dialog-centered modal-dialog-scrollable">
                            <form class="modal-content" onsubmit={on_submit_form}>
                                <div class="modal-header">
                                    <h5 class="modal-title">{"新增服务器"}</h5>
                                    <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close"></button>
                                </div>
                                <div class="modal-body">
                                    <div class="row mb-3">
                                        {
                                            if let SubmitStatus::Error{ err } = &self.submit_status {
                                                html! {
                                                    <div class="alert alert-danger" role="alert">
                                                        {"提交失败："} {err.to_string()}
                                                    </div>
                                                }
                                            } else {
                                                html! {}
                                            }
                                        }
                                    </div>

                                    <div class="row mb-3">
                                        <label for="name" class="col-sm-4 col-form-label">
                                            {"服务器名"}
                                        </label>
                                        <div class="col-sm-8">
                                            <input type="text" class="form-control" id="name" maxlength={100} required={true} ref={self.create_server_config_name.clone()} />
                                        </div>
                                    </div>

                                    <div class="row mb-3">
                                        <label for="version" class="col-sm-4 col-form-label">
                                            {"服务器版本"}
                                        </label>
                                        <div class="col-sm-8">
                                            <select class="form-control form-select mb-3" id="version" required={true} ref={self.create_server_config_version.clone()}>
                                                {
                                                    self.mc_versions.iter().map(|version| {
                                                        html! {
                                                            <option value={version.clone()}>{version.clone()}</option>
                                                        }
                                                    }).collect::<Html>()
                                                }
                                            </select>
                                            <div class="form-check">
                                                <input type="checkbox" class="form-check-input" id="version-has-snapshot" checked={self.with_snapshot_version} onchange={on_change_with_snapshot} />
                                                <label for="version-has-snapshot">{"包含快照版"}</label>
                                            </div>
                                        </div>
                                    </div>

                                    <div class="row mb-3">
                                        <label for="world" class="col-sm-4 col-form-label">
                                            {"预设存档"}
                                            {
                                                if matches!(self.upload_world, UploadUriStatus::Uploading) {
                                                    html! {
                                                        <div class="spinner-border spinner-border-sm" role="status">
                                                            <span class="visually-hidden">{"上传中"}</span>
                                                        </div>
                                                    }
                                                } else {
                                                    html!{}
                                                }
                                            }
                                        </label>
                                        <div class="col-sm-8">
                                            <input type="file" class="form-control" id="world" accept=".zip"
                                                onchange={world_file_select} />
                                        </div>
                                    </div>

                                    <div class="row mb-3">
                                        {
                                            if let UploadUriStatus::Error { err } = &self.upload_world {
                                                html! {
                                                    <p style="color: red">
                                                        {"上传文件失败，请重新选择文件后重试："} {err.clone()}
                                                    </p>
                                                }
                                            } else {
                                                html!{}
                                            }
                                        }
                                    </div>

                                    <div class="row mb-3">
                                        <label for="resource" class="col-sm-4 col-form-label">
                                            {"服务端资源包"}
                                            {
                                                if matches!(self.upload_resource, UploadUriStatus::Uploading) {
                                                    html! {
                                                        <div class="spinner-border spinner-border-sm" role="status">
                                                            <span class="visually-hidden">{"上传中"}</span>
                                                        </div>
                                                    }
                                                } else {
                                                    html!{}
                                                }
                                            }
                                        </label>
                                        <div class="col-sm-8">
                                            <input type="file" class="form-control" id="resource" accept=".zip"
                                                onchange={resource_file_select} />
                                        </div>
                                    </div>

                                    <div class="row mb-3">
                                        {
                                            if let UploadUriStatus::Error { err } = &self.upload_resource {
                                                html! {
                                                    <p style="color: red">
                                                        {"上传文件失败，请重新选择文件后重试："} {err.clone()}
                                                    </p>
                                                }
                                            } else {
                                                html!{}
                                            }
                                        }
                                    </div>

                                    <div class="row mb-3">
                                        <label for="motd" class="col-sm-4 col-form-label">
                                            {"MOTD"}
                                        </label>
                                        <div class="col-sm-8">
                                            <textarea class="form-control" id="motd" ref={self.create_server_config_motd.clone()} />
                                            <p>
                                                {"可通过"}
                                                <a href="https://zh.minecraft.wiki/w/%E6%A0%BC%E5%BC%8F%E5%8C%96%E4%BB%A3%E7%A0%81?variant=zh-cn#%E7%BC%96%E8%BE%91%E5%99%A8" target="_blank">
                                                    {"格式化代码编辑器"}
                                                </a>
                                                {"编辑样式"}
                                            </p>
                                        </div>
                                    </div>
                                </div>
                                <div class="modal-footer">
                                    <button type="button" class="btn btn-secondary" data-bs-dismiss="modal">{"取消"}</button>
                                    <button type="submit" class="btn btn-primary"
                                        disabled={!self.upload_world.is_can_submit() || !self.upload_resource.is_can_submit()}>
                                        {"提交"}
                                        {
                                            if matches!(self.submit_status, SubmitStatus::Submitting) {
                                                html! {
                                                    <div class="spinner-border text-primary" role="status">
                                                        <span class="visually-hidden">{"Submitting"}</span>
                                                    </div>
                                                }
                                            } else {
                                                html!{}
                                            }
                                        }
                                    </button>
                                </div>
                            </form>
                        </div>
                    </div>
                </div>
            </>
        }
    }
}

#[derive(Eq, PartialEq, Properties)]
pub struct ProgressNodeProps {
    title: AttrValue,
    has_next: bool,
    icon: ProgressIcon,
    start_time: Option<i64>,
    end_time: Option<i64>,
}

#[derive(PartialEq, Eq)]
enum ProgressIcon {
    Running,
    Pass,
    Error,
    Waiting,
}

#[function_component]
pub fn ProgressNode(props: &ProgressNodeProps) -> Html {
    let runtime = use_state(|| None);

    {
        let start_time = props.start_time.clone();
        let end_time = props.end_time.clone();
        let runtime = runtime.clone();
        use_effect_with((start_time, end_time), move |(start_time, end_time)| {
            let mut interval = None;

            if let Some(start_time) = start_time {
                if let Some(end_time) = end_time {
                    runtime.set(Some(end_time - start_time))
                } else {
                    let start_time = *start_time;
                    interval = Some(Interval::new(1000, move || {
                        let duration = js_sys::Date::new_0().get_time() as i64 / 1000 - start_time;
                        runtime.set(Some(duration));
                    }));
                }
            }

            || drop(interval)
        });
    }

    let runtime_text = AttrValue::from(
        runtime
            .map(|time| match time {
                time if time >= 60 * 60 * 24 => format!(
                    "已运行 {} 天 {} 时 {} 分 {} 秒",
                    time / 60 / 60 / 24,
                    time / 60 / 60 % 24,
                    time / 60 % 60,
                    time % 60
                ),
                time if time >= 60 * 60 => format!(
                    "已运行 {} 时 {} 分 {} 秒",
                    time / 60 / 60,
                    time / 60 % 60,
                    time % 60
                ),
                time if time >= 60 => format!("已运行 {} 分 {} 秒", time / 60, time % 60),
                time => format!("已运行 {} 秒", time),
            })
            .unwrap_or("等待中".to_owned()),
    );

    html! {
        <div class="col">
            <div class="card" style="width: 250px;">
                <div class="row g-0">
                    <div class="card-header col-4"
                        style={match props.icon {
                            ProgressIcon::Running => "text-align: center;",
                            ProgressIcon::Pass => "background: #90ee90;",
                            ProgressIcon::Error => "background: #ee9090",
                            ProgressIcon::Waiting => "",
                        }}
                    >
                        {
                            match props.icon {
                                ProgressIcon::Running => html! {
                                    <div class="spinner-border text-primary" role="status" style="margin-top: 50%; margin-bottom: 50%;">
                                        <span class="visually-hidden">{"Running"}</span>
                                    </div>
                                },
                                ProgressIcon::Pass => html! {
                                    <div style="margin-top: 50%; margin-bottom: 50%;">
                                        <i class="bi bi-check-circle" style="display: inline-block; text-align: center; width: 100%; font-size: 24px; color: green"></i>
                                    </div>
                                },
                                ProgressIcon::Error => html!{
                                    <div style="margin-top: 50%; margin-bottom: 50%;">
                                        <i class="bi bi-x-circle" style="display: inline-block; text-align: center; width: 100%; font-size: 24px; color: red"></i>
                                    </div>
                                },
                                ProgressIcon::Waiting => html!{
                                    <div style="margin-top: 50%; margin-bottom: 50%;">
                                        <i class="bi bi-dash-circle" style="display: inline-block; text-align: center; width: 100%; font-size: 24px; color: gray"></i>
                                    </div>
                                },
                            }
                        }
                    </div>
                    <div class="card-body col-8">
                        <p class="card-title" title={props.title.clone()}
                            style="text-overflow: ellipsis; white-space: nowrap; overflow: hidden;">
                            {props.title.clone()}
                        </p>
                        <p class="card-text" title={runtime_text.clone()}
                            style="text-overflow: ellipsis; white-space: nowrap; overflow: hidden;">
                            <small style="text-muted">{runtime_text.clone()}</small>
                        </p>
                    </div>
                </div>
            </div>
            {
                if props.has_next {
                    html! {<div style="position: relative; width: 200px; height: 2px; background: gray; left: 250px; bottom: 50%;"></div>}
                } else {
                    html! {<></>}
                }
            }
        </div>
    }
}

#[derive(PartialEq, Properties)]
pub struct ServerConfigProps {
    id: u64,
    name: String,
    version: String,
    motd: String,
    onstart: Callback<u64>,
}

#[function_component]
pub fn ServerConfigNode(props: &ServerConfigProps) -> Html {
    let ServerConfigProps {
        id,
        name,
        version,
        motd,
        onstart,
    } = props;

    let on_start_click = {
        let id = *id;
        let onstart = onstart.clone();
        Callback::from(move |_: MouseEvent| onstart.emit(id))
    };

    html! {
        <tr>
            <th scope="row">{id}</th>
            <td>{name}</td>
            <td>{version}</td>
            <td>{motd}</td>
            <td>
                <div class="btn-group" role="group">
                    <button type="button" class="btn btn-sm btn-outline-success" onclick={on_start_click}>{"启动"}</button>
                </div>
            </td>
        </tr>
    }
}

#[function_component]
pub fn ServerConfigPlaceHolder() -> Html {
    html! {
        <tr>
            <th scope="row" class="placeholder-glow"><span class="placeholder">{0}</span></th>
            <td class="placeholder-glow"><span class="placeholder">{"name"}</span></td>
            <td class="placeholder-glow"><span class="placeholder">{"1.xx.x"}</span></td>
            <td class="placeholder-glow"><span class="placeholder">{"A Minecraft Server"}</span></td>
            <td class="placeholder-glow">
                <span class="placeholder">
                    <div class="btn-group invisible" role="group">
                        <button type="button" class="btn btn-sm btn-outline-success" disabled={true}>{"启动"}</button>
                    </div>
                </span>
            </td>
        </tr>
    }
}
