use anyhow::{anyhow, Result};
use bilibili_api::bangumi::BangumiClient;
use bilibili_api::Client;
use bytes::Bytes;
use std::collections::HashMap;
use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use web_db::bilibili::{get_all_bangumi_watch, update_send_ep_and_query_time, BangumiWatch};
use web_db::{begin_tx, create_connection, Transaction, RDS};

pub async fn handle() -> Result<()> {
    let mut conn = create_connection(RDS::Bilibili).await?;
    let mut tx = begin_tx(&mut conn).await?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

    let bangumi_watches = get_all_bangumi_watch(&mut tx).await?;

    let bilibili_client = build_bilibili_client()?;

    let mut fetch_error = false;
    for mut bangumi_watch in bangumi_watches {
        if bangumi_watch.finish || bangumi_watch.next_query_time > now {
            continue;
        }

        if let Err(err) = fetch_and_notify(&mut tx, &bilibili_client, &mut bangumi_watch).await {
            fetch_error = true;
            log::error!(
                "fetch-bilibili-bangumi error: {err}, rowid={}, ssid={}",
                bangumi_watch.rowid,
                bangumi_watch.ssid,
            );
        }
    }

    if fetch_error {
        Err(anyhow!(
            "fetch-bilibili-bangumi error, see logs to see detali"
        ))
    } else {
        Ok(())
    }
}

fn build_bilibili_client() -> Result<Client> {
    Ok(serde_json::from_str(include_str!(
        "../../config/bilibili_client.json"
    ))?)
}

async fn fetch_and_notify(
    tx: &mut Transaction<'_>,
    client: &Client,
    bangumi_watch: &mut BangumiWatch,
) -> Result<()> {
    let bangumi_info = client.get_web_season(bangumi_watch.ssid).await?;

    if bangumi_info.episodes.len() <= bangumi_watch.send_ep as usize {
        log::info!(
            "bangumi {} queried {} episodes, skip...",
            bangumi_watch.ssid,
            bangumi_info.episodes.len()
        );
        return Ok(());
    }

    let ep = bangumi_info
        .episodes
        .last()
        .ok_or(anyhow!("episodes is empty"))?;
    let url = client.get_video_stream_url(ep.id).await?;

    if url.dash.video.is_empty() || url.dash.audio.is_empty() {
        return Err(anyhow!("video or audio url is empty"));
    }

    log::info!(
        "get video url success: {} / {}",
        url.dash.video[0].base_url,
        url.dash.audio[0].base_url
    );

    tokio::try_join!(
        download_file_and_save(
            "/tmp/bilibili-video",
            &url.dash.video[0].base_url,
            &url.dash.video[0].backup_url,
        ),
        download_file_and_save(
            "/tmp/bilibili-audio",
            &url.dash.audio[0].base_url,
            &url.dash.audio[0].backup_url,
        )
    )?;

    log::info!("video & audio downloaded");

    let convert_success = std::process::Command::new("ffmpeg")
        .args(vec![
            "-y",
            "-i",
            "/tmp/bilibili-video",
            "-i",
            "/tmp/bilibili-audio",
            "-c:v",
            "copy",
            "-c:a",
            "aac",
            "/tmp/bilibili-output",
        ])
        .spawn()?
        .wait()?
        .success();
    if !convert_success {
        return Err(anyhow!("call ffmpeg fail"));
    }

    log::info!("video & audio converted");

    let mut video_data = Vec::new();
    tokio::fs::File::open("/tmp/bilibili-output")
        .await?
        .read_to_end(&mut video_data)
        .await?;

    let expire_at = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .add(Duration::from_secs(60 * 60 * 24 * 7))
        .as_secs();
    let oss_key = format!(
        "rust-web/bili-video/{}.{}.{}.mp4",
        bangumi_watch.ssid, ep.aid, expire_at
    );
    aliyun_helper::oss::upload_file(&oss_key, video_data).await?;
    let preview_url = aliyun_helper::oss::get_download_url(&oss_key, expire_at);
    log::info!("upload oss, url={preview_url}");

    let cover_data = download_file(&ep.cover).await?;
    log::info!("cover download success, url={}", &ep.cover);

    tokio::fs::File::create("image.png")
        .await?
        .write(&cover_data)
        .await?;

    let cover_key = feishu_api::api::message::upload_image(cover_data.to_vec())
        .await?
        .data
        .image_key;

    log::info!("upload image to feishu");

    let mut params = HashMap::new();
    params.insert("fan_title".to_string(), bangumi_info.season_title);
    params.insert("ep_title".to_string(), ep.share_copy.clone());
    params.insert("cover".to_string(), cover_key);
    params.insert("url".to_string(), preview_url);
    feishu_api::sdk::send_card_message("ctp_AA85tBfL0sMR", params).await?;

    log::info!("send feishu card message");

    bangumi_watch.send_ep = bangumi_info.episodes.len() as i32;
    bangumi_watch.next_query_time += 60 * 60 * 24 * 7;
    update_send_ep_and_query_time(tx, bangumi_watch).await?;

    log::info!("save changes to db");
    Ok(())
}

async fn download_file(url: &str) -> Result<Bytes> {
    Ok(reqwest::Client::new()
        .get( url)
        .header("Referer", "https://www.bilibili.com")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/118.0.0.0 Safari/537.36 Edg/118.0.2088.76")
        .send()
        .await?
        .bytes()
        .await?)
}

async fn download_file_and_save(
    save_path: &str,
    base_url: &str,
    _backup_url: &Vec<String>,
) -> Result<()> {
    let bytes = download_file(base_url).await?;

    println!("download bytes: {}", bytes.len());

    tokio::fs::File::create(save_path)
        .await?
        .write_all(&bytes)
        .await?;

    Ok(())
}
