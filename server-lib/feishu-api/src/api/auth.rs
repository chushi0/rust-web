use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GetTenantAccessTokenRequest {
    pub app_id: String,
    pub app_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GetTenantAccessTokenResponse {
    pub code: i32,
    pub msg: String,
    pub tenant_access_token: String,
    pub expire: u64,
}

#[derive(Debug)]
pub struct TenantAccessToken {
    pub token: String,
    pub expire: u64,
}

const APP_ID: &'static str = env!("RUST_WEB_FEISHU_APP_ID");
const APP_SECRET: &'static str = env!("RUST_WEB_FEISHU_APP_SECRET");

pub async fn get_tenant_access_token() -> Result<TenantAccessToken> {
    let url = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal";

    let request_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("get system time fail");

    let client = reqwest::Client::new();
    let resp: GetTenantAccessTokenResponse = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&GetTenantAccessTokenRequest {
            app_id: APP_ID.to_string(),
            app_secret: APP_SECRET.to_string(),
        })?)
        .send()
        .await?
        .json()
        .await?;

    if resp.code != 0 {
        return Err(anyhow!("code is not zero: {}", resp.code));
    }

    Ok(TenantAccessToken {
        token: resp.tenant_access_token,
        expire: resp.expire + request_time.as_secs(),
    })
}

#[test]
pub fn test_get_tenant_access_token() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let token = get_tenant_access_token().await;
            println!("token: {token:?}")
        })
}
