use anyhow::{anyhow, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, BufReader};

const SERVER_DIR: &'static str = "/home/chushi0/mc/server/1.20/";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCache {
    name: String,
    uuid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Advancement {
    pub criteria: HashMap<String, String>,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum AdvancementFile {
    Advancement(Advancement),
    Number(i32),
}

pub async fn get_player_uuid(name: &str) -> Result<String> {
    let users: Vec<UserCache> = read_mc_file("usercache.json").await?;
    for user_cache in &users {
        if user_cache.name == name {
            return Ok(user_cache.uuid.clone());
        }
    }

    Err(anyhow!("user {name} not found"))
}

pub async fn get_player_advancement(uuid: &str) -> Result<HashMap<String, Advancement>> {
    let file: HashMap<String, AdvancementFile> =
        read_mc_file(format!("world/advancements/{uuid}.json")).await?;

    let mut res = HashMap::new();
    for (id, file) in file {
        match file {
            AdvancementFile::Advancement(v) => {
                res.insert(id, v);
            }
            AdvancementFile::Number(_) => (),
        };
    }
    Ok(res)
}

async fn read_mc_file<T, S>(path: S) -> Result<T>
where
    T: DeserializeOwned,
    S: Into<String>,
{
    let file = SERVER_DIR.to_owned() + &path.into();
    let file = tokio::fs::File::open(file).await?;
    let mut reader = BufReader::new(file);
    let mut data = Vec::new();
    reader.read_to_end(&mut data).await?;

    Ok(serde_json::from_slice(data.as_slice())?)
}
