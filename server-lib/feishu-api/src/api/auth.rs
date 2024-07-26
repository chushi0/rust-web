use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GetTenantAccessTokenRequest {
    pub app_id: &'static str,
    pub app_secret: &'static str,
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

static APP_ID: OnceLock<&'static str> = OnceLock::new();
static APP_SECRET: OnceLock<&'static str> = OnceLock::new();

pub(crate) fn get_app_id() -> &'static str {
    APP_ID.get_or_init(|| std::env::var("RUST_WEB_FEISHU_APP_ID").unwrap().leak())
}

pub(crate) fn get_app_secret() -> &'static str {
    APP_SECRET.get_or_init(|| std::env::var("RUST_WEB_FEISHU_APP_SECRET").unwrap().leak())
}

pub async fn get_tenant_access_token() -> Result<TenantAccessToken> {
    get_tenant_access_token_internal(super::FEISHU_HOST).await
}

async fn get_tenant_access_token_internal(host: &str) -> Result<TenantAccessToken> {
    let url = format!("{}/open-apis/auth/v3/tenant_access_token/internal", host);

    let request_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("get system time fail");

    let client = reqwest::Client::new();
    let resp: GetTenantAccessTokenResponse = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(serde_json::to_vec(&GetTenantAccessTokenRequest {
            app_id: get_app_id(),
            app_secret: get_app_secret(),
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

#[cfg(test)]
mod test {
    use super::*;
    use mockito::Matcher;
    use serde_json::json;

    #[test]
    pub fn test_get_tenant_access_token_success() {
        let mut server = mockito::Server::new();

        tokio_test::block_on(async {
            let expect_body = Matcher::Json(json!({
                "app_id": get_app_id(),
                "app_secret": get_app_secret()
            }));

            let expect_token = "t-caecc734c2e3328a62489fe0648c4b98779515d3";
            let expect_expire = 7200;

            let response_body = json!({
                "code": 0,
                "msg": "ok",
                "tenant_access_token": expect_token,
                "expire": expect_expire
            })
            .to_string();

            let mock = server
                .mock("POST", "/open-apis/auth/v3/tenant_access_token/internal")
                .match_header("Content-Type", "application/json")
                .match_body(expect_body)
                .with_header("Content-Type", "application/json")
                .with_body(response_body)
                .create();

            let before_call_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("get current time failed")
                .as_secs();

            let token = get_tenant_access_token_internal(&server.url())
                .await
                .expect("this http request should return success");

            let after_call_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("get current time failed")
                .as_secs();

            assert_eq!(token.token, expect_token);
            assert!(before_call_time + expect_expire <= token.expire);
            assert!(after_call_time + expect_expire >= token.expire);
            mock.assert();
        })
    }
}
