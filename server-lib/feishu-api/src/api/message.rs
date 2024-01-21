use anyhow::{anyhow, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    multipart::{Form, Part},
};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum ReceiveIdType {
    OpenId,
    UserId,
    UnionId,
    Email,
    ChatId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub receive_id: String,
    pub msg_type: String,
    pub content: String,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageToHookRequest {
    pub msg_type: String,
    pub card: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageResponse {
    pub code: i32,
    pub msg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadImageResponse {
    pub code: i32,
    pub msg: String,
    pub data: UploadImageResponsePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadImageResponsePayload {
    pub image_key: String,
}

impl From<ReceiveIdType> for &'static str {
    fn from(value: ReceiveIdType) -> Self {
        match value {
            ReceiveIdType::OpenId => "open_id",
            ReceiveIdType::UserId => "user_id",
            ReceiveIdType::UnionId => "union_id",
            ReceiveIdType::Email => "email",
            ReceiveIdType::ChatId => "chat_id",
        }
    }
}

pub async fn send_message(
    receive_id_type: ReceiveIdType,
    req: SendMessageRequest,
) -> Result<SendMessageResponse> {
    send_message_internal(super::FEISHU_HOST, receive_id_type, req).await
}

async fn send_message_internal(
    host: &str,
    receive_id_type: ReceiveIdType,
    req: SendMessageRequest,
) -> Result<SendMessageResponse> {
    let url = format!(
        "{}/open-apis/im/v1/messages?receive_id_type={}",
        host,
        Into::<&'static str>::into(receive_id_type)
    );
    let token = crate::get_token().await.ok_or(anyhow!("get token fail"))?;
    let token = format!("Bearer {token}");
    println!("token: {token}");

    let client = reqwest::Client::new();
    let resp: SendMessageResponse = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", token)
        .body(serde_json::to_vec(&req)?)
        .send()
        .await?
        .json()
        .await?;

    if resp.code != 0 {
        return Err(anyhow!(
            "code is not zero: {}, msg: {}",
            resp.code,
            resp.msg
        ));
    }

    Ok(resp)
}

pub async fn send_message_to_webhook(
    hook_id: &str,
    req: SendMessageToHookRequest,
) -> Result<SendMessageResponse> {
    let url = format!("https://open.feishu.cn/open-apis/bot/v2/hook/{}", hook_id);
    let token = crate::get_token().await.ok_or(anyhow!("get token fail"))?;
    let token = format!("Bearer {token}");
    println!("token: {token}");

    let client = reqwest::Client::new();
    let resp: SendMessageResponse = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", token)
        .body(serde_json::to_vec(&req)?)
        .send()
        .await?
        .json()
        .await?;

    if resp.code != 0 {
        return Err(anyhow!(
            "code is not zero: {}, msg: {}",
            resp.code,
            resp.msg
        ));
    }

    Ok(resp)
}

pub async fn upload_image(image: Vec<u8>) -> Result<UploadImageResponse> {
    let token = crate::get_token().await.ok_or(anyhow!("get token fail"))?;
    let token = format!("Bearer {token}");
    println!("token: {token}");

    let mut image_header = HeaderMap::new();
    image_header.insert("Content-Type", HeaderValue::from_static("image/png"));
    let form = Form::new().text("image_type", "message").part(
        "image",
        Part::bytes(image).file_name("image").headers(image_header),
    );

    let client = reqwest::Client::new();
    let resp: UploadImageResponse = client
        .post("https://open.feishu.cn/open-apis/im/v1/images")
        .header("Authorization", token)
        .multipart(form)
        .send()
        .await?
        .json()
        .await?;

    if resp.code != 0 {
        return Err(anyhow!(
            "code is not zero: {}, msg: {}",
            resp.code,
            resp.msg
        ));
    }

    Ok(resp)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::api::USER_ID;
    use crate::install_test_key;
    use mockito::Matcher;
    use serde_json::json;

    #[test]
    pub fn test_send_message() {
        tokio_test::block_on(async {
            let mock_token = "t-caecc734c2e3328a62489fe0648c4b98779515d3";
            install_test_key(mock_token).await;

            let mut server = mockito::Server::new();

            let expect_body = Matcher::Json(json!({
                "receive_id": USER_ID,
                "msg_type": "text",
                "content": "{\"text\":\"test content\"}",
                "uuid": null,
            }));

            let response_body = json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "message_id": "om_dc13264520392913993dd051dba21dcf",
                    "root_id": "om_40eb06e7b84dc71c03e009ad3c754195",
                    "parent_id": "om_d4be107c616aed9c1da8ed8068570a9f",
                    "thread_id": "omt_d4be107c616a",
                    "msg_type": "card",
                    "create_time": "1615380573411",
                    "update_time": "1615380573411",
                    "deleted": false,
                    "updated": false,
                    "chat_id": "oc_5ad11d72b830411d72b836c20",
                    "sender": {
                        "id": "cli_9f427eec54ae901b",
                        "id_type": "app_id",
                        "sender_type": "app",
                        "tenant_key": "736588c9260f175e"
                    },
                    "body": {
                        "content": "{\"text\":\"@_user_1 test content\"}"
                    },
                    "mentions": [
                        {
                            "key": "@_user_1",
                            "id": "ou_155184d1e73cbfb8973e5a9e698e74f2",
                            "id_type": "open_id",
                            "name": "Tom",
                            "tenant_key": "736588c9260f175e"
                        }
                    ],
                    "upper_message_id": "om_40eb06e7b84dc71c03e009ad3c754195"
                }
            })
            .to_string();

            let mock = server
                .mock("POST", "/open-apis/im/v1/messages")
                .match_query("receive_id_type=open_id")
                .match_header(
                    "Authorization",
                    "Bearer t-caecc734c2e3328a62489fe0648c4b98779515d3",
                )
                .match_header("Content-Type", "application/json")
                .match_body(expect_body)
                .with_header("Content-Type", "application/json")
                .with_body(response_body)
                .create();

            let res = send_message_internal(
                &server.url(),
                ReceiveIdType::OpenId,
                SendMessageRequest {
                    receive_id: USER_ID.to_string(),
                    msg_type: "text".to_string(),
                    content: "{\"text\":\"test content\"}".to_string(),
                    uuid: None,
                },
            )
            .await
            .expect("this http request should return success");

            assert_eq!(res.code, 0);
            mock.assert();
        });
    }
}
