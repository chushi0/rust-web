use clap::Parser;

pub mod api;
pub mod biz;

#[derive(Parser, Debug)]
enum ProgramArgs {
    FetchGithubActivity,
    RefreshMcAdvancement { path: String, lang: String },
    FetchBilibiliBangumi,
    FetchBilibiliBangumiAll { ssid: i32 },
}

#[tokio::main]
async fn main() {
    log4rs::init_file("conf/log4rs.yaml", Default::default()).unwrap();

    let arg = ProgramArgs::parse();
    log::info!("starting cronjob: {arg:?}");
    let result = match arg {
        ProgramArgs::FetchGithubActivity => biz::fetch_github_activity::handle().await,
        ProgramArgs::RefreshMcAdvancement { path, lang } => {
            biz::refresh_mc_advancement::handle(&path, &lang).await
        }
        ProgramArgs::FetchBilibiliBangumi => biz::fetch_bilibili_bangumi::handle().await,
        ProgramArgs::FetchBilibiliBangumiAll { ssid } => {
            biz::fetch_bilibili_bangumi::handle_all(ssid).await
        }
    };

    if let Err(e) = result {
        log::error!("execute cronjob fail: {e} {}", e.backtrace());
    }
}
