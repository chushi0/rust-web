use anyhow::{anyhow, Result};
use base64::Engine;
use reqwest::StatusCode;
use std::ops::Add;
use std::time::{Duration, SystemTime};

pub fn get_download_url(path: &str, expire_at: u64) -> String {
    let bucket_host = include_str!("../config/bucket_host.txt");
    let access_key_id = include_str!("../config/access_key_id.txt");
    let bucket_name = include_str!("../config/bucket_name.txt");
    let access_key_secret = include_str!("../config/access_key_secret.txt");
    let plain_message = format!("GET\n\n\n{}\n/{}/{}", expire_at, bucket_name, path);

    let signature = hmacsha1::hmac_sha1(access_key_secret.as_bytes(), plain_message.as_bytes());
    let signature = base64::engine::general_purpose::STANDARD.encode(&signature);
    let signature = urlencoding::encode(&signature);

    format!(
        "https://{}/{}?OSSAccessKeyId={}&Expires={}&Signature={}",
        bucket_host, path, access_key_id, expire_at, signature
    )
}

pub async fn upload_file(path: &str, file: Vec<u8>, mimetype: &str) -> Result<()> {
    let bucket_host = include_str!("../config/bucket_host.txt");
    let access_key_id = include_str!("../config/access_key_id.txt");
    let bucket_name = include_str!("../config/bucket_name.txt");
    let access_key_secret = include_str!("../config/access_key_secret.txt");

    let expire_at = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .add(Duration::from_secs(3600 * 2))
        .as_secs();
    let plain_message = format!(
        "PUT\n\n{}\n{}\n/{}/{}",
        mimetype, expire_at, bucket_name, path
    );

    let signature = hmacsha1::hmac_sha1(access_key_secret.as_bytes(), plain_message.as_bytes());
    let signature = base64::engine::general_purpose::STANDARD.encode(&signature);
    let signature = urlencoding::encode(&signature);

    let url = format!(
        "https://{}/{}?OSSAccessKeyId={}&Expires={}&Signature={}",
        bucket_host, path, access_key_id, expire_at, signature
    );

    let code = reqwest::Client::new()
        .put(url)
        .header("Content-Type", mimetype)
        .body(file)
        .send()
        .await?
        .status();

    if code == StatusCode::OK || code == StatusCode::CREATED || code == StatusCode::ACCEPTED {
        Ok(())
    } else {
        Err(anyhow!("status not 200/201/202: {code}"))
    }
}