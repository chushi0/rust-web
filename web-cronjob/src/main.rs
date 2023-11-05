use clap::Parser;

pub mod api;
pub mod biz;

#[derive(Parser, Debug)]
enum ProgramArgs {
    FetchGithubActivity,
    RefreshMcAdvancement { path: String, lang: String },
    FetchBilibiliBangumi,
}

#[tokio::main]
async fn main() {
    if cfg!(debug_assertions) {
        log4rs::init_file("log4rs.debug.yaml", Default::default()).unwrap();
    } else {
        log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    }

    let arg = ProgramArgs::parse();
    log::info!("starting cronjob: {arg:?}");
    let result = match arg {
        ProgramArgs::FetchGithubActivity => biz::fetch_github_activity::handle().await,
        ProgramArgs::RefreshMcAdvancement { path, lang } => {
            biz::refresh_mc_advancement::handle(&path, &lang).await
        }
        ProgramArgs::FetchBilibiliBangumi => biz::fetch_bilibili_bangumi::handle().await,
    };

    if let Err(e) = result {
        log::error!("execute cronjob fail: {e}");
    }
}
