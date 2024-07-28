use std::time::{Duration, SystemTime};

use crate::model::Model;
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct OssFileObtainResp {
    pub uri: String,
    pub url: String,
}

pub async fn oss_file_obtain(uri: String) -> Result<Model<OssFileObtainResp>> {
    if !uri.starts_with("game_assets/") || uri.contains("..") {
        return Ok(Model {
            code: 403,
            msg: "Access denied".to_string(),
            data: None,
        });
    }

    let time =
        SystemTime::now().duration_since(std::time::UNIX_EPOCH)? + Duration::from_secs(2 * 60 * 60);

    let url = aliyun_helper::oss::get_download_url(&uri, time.as_secs());
    Ok(Model::from_success(OssFileObtainResp { uri, url }))
}
