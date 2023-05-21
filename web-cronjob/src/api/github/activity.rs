use std::rc::Rc;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Rc<String>,
    pub actor: Actor,
    #[serde(rename = "repo")]
    pub repository: Repository,
    #[serde(flatten)]
    pub payload: EventPayload,
    pub public: bool,
    #[serde(with = "time::serde::iso8601")]
    pub created_at: time::OffsetDateTime,
    pub org: Option<Organization>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub id: i64,
    #[serde(rename = "login")]
    pub username: Rc<String>,
    #[serde(rename = "display_login")]
    pub display_username: Rc<String>,
    pub gravatar_id: Rc<String>,
    pub url: Rc<String>,
    pub avatar_url: Rc<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: i64,
    pub name: Rc<String>,
    pub url: Rc<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: i64,
    #[serde(rename = "login")]
    pub name: Rc<String>,
    pub gravatar_id: Rc<String>,
    pub url: Rc<String>,
    pub avatar_url: Rc<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EventPayload {
    ForkEvent {
        payload: ForkEventPayload,
    },
    PublicEvent,
    PushEvent {
        payload: PushEventPayload,
    },
    WatchEvent {
        payload: WatchEventPayload,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkEventPayload {
    pub forkee: super::repository::Repository,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushEventPayload {
    pub push_id: i64,
    pub size: i32,
    pub distinct_size: i32,
    #[serde(rename = "ref")]
    pub git_ref: Rc<String>,
    #[serde(rename = "head")]
    pub head_sha: Rc<String>,
    #[serde(rename = "before")]
    pub before_sha: Rc<String>,
    pub commits: Vec<Commit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchEventPayload {
    pub action: Rc<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub sha: Rc<String>,
    pub message: Rc<String>,
    pub author: Author,
    pub distinct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: Rc<String>,
    pub email: Rc<String>,
    pub url: Option<Rc<String>>,
}

pub async fn list_user_public_events(
    username: &str,
    per_page: i32,
    page: i32,
) -> Result<Vec<Event>> {
    log::info!("[Github API] list_user_public_events, username={username}, per_page={per_page}, page={page}");
    let url = format!(
        "https://api.github.com/users/{username}/events/public?per_page={per_page}&page={page}"
    );
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header(reqwest::header::USER_AGENT, crate::api::UA)
        .send()
        .await?;

    if cfg!(debug_assertions) {
        let raw_data = resp.text().await?;
        log::debug!("[Github API] list_user_public_events, resp={raw_data}");

        Ok(serde_json::from_str(&raw_data)?)
    } else {
        Ok(resp.json::<Vec<Event>>().await?)
    }
}
