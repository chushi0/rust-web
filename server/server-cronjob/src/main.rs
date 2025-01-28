use clap::Parser;

pub mod api;
pub mod biz;

#[derive(Parser, Debug)]
enum ProgramArgs {
    FetchGithubActivity,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let arg = ProgramArgs::parse();
    log::info!("starting cronjob: {arg:?}");
    let result = match arg {
        ProgramArgs::FetchGithubActivity => biz::fetch_github_activity::handle().await,
    };

    if let Err(e) = result {
        log::error!("execute cronjob fail: {e:?}");
    }
}
