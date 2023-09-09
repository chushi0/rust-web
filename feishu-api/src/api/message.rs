use anyhow::{anyhow, Result};
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
pub struct SendMessageResponse {
    pub code: i32,
    pub msg: String,
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
    let url = format!(
        "https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type={}",
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

#[test]
pub fn test_get_tenant_access_token() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let res = send_message(
                ReceiveIdType::OpenId,
                SendMessageRequest {
                    receive_id: super::USER_ID.to_string(),
                    msg_type: "text".to_string(),
                    content: "{\"text\":\"test_content\"}".to_string(),
                    uuid: None,
                },
            )
            .await;
            println!("{res:#?}");
        })
}
