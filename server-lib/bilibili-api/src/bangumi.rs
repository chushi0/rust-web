use crate::{internal::Model, Client};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait BangumiClient {
    async fn get_web_season(&self, season_id: i32) -> Result<WebSeasonResponse>;
    async fn get_video_stream_url(&self, ep_id: i32) -> Result<VideoStreamUrlResponse>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSeasonResponse {
    pub cover: String,
    pub episodes: Vec<Ep>,
    pub season_id: i32,
    pub season_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStreamUrlResponse {
    pub dash: VideoStreamDashUrl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStreamDashUrl {
    pub duration: i32,
    pub video: Vec<VideoDashUrl>,
    pub audio: Vec<AudioDashUrl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoDashUrl {
    pub base_url: String,
    pub backup_url: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDashUrl {
    pub base_url: String,
    pub backup_url: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ep {
    pub aid: i64,
    pub bvid: String,
    pub cid: i32,
    pub cover: String,
    pub id: i32, // epid
    pub long_title: String,
    pub pub_time: i64,
    pub title: String,
    pub subtitle: String,
    pub vid: String,
    pub share_copy: String,
}

#[async_trait]
impl BangumiClient for Client {
    async fn get_web_season(&self, season_id: i32) -> Result<WebSeasonResponse> {
        let url = format!(
            "https://api.bilibili.com/pgc/view/web/season?season_id={}",
            season_id
        );
        let resp: Model<_> = self
            .request_with_auth(reqwest::Client::new().get(url))
            .send()
            .await?
            .json()
            .await?;

        if resp.code != 0 {
            return Err(anyhow!("response code != 0: code={}", resp.code));
        }

        Ok(resp.result)
    }

    async fn get_video_stream_url(&self, ep_id: i32) -> Result<VideoStreamUrlResponse> {
        let url = format!(
            "https://api.bilibili.com/pgc/player/web/playurl?ep_id={}&qn=120&fnval=16",
            ep_id
        );
        let resp: Model<_> = self
            .request_with_auth(reqwest::Client::new().get(url))
            .header("Referer", "https://www.bilibili.com/")
            .send()
            .await?
            .json()
            .await?;

        if resp.code != 0 {
            return Err(anyhow!("response code != 0: code={}", resp.code));
        }

        Ok(resp.result)
    }
}
