use std::collections::HashMap;

use crate::api::{self, message::SendMessageRequest};
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct PlainMessage<'a> {
    pub text: &'a str,
}

#[derive(Debug, Serialize)]
struct CardMessage<'a> {
    pub r#type: &'a str,
    pub data: CardMessageData<'a>,
}

#[derive(Debug, Serialize)]
struct CardMessageData<'a> {
    pub template_id: &'a str,
    pub template_variable: HashMap<String, String>,
}

pub async fn send_plain_message<'a>(content: &'a str) -> Result<()> {
    let content = serde_json::to_string(&PlainMessage { text: content })?;

    api::message::send_message(
        api::message::ReceiveIdType::OpenId,
        SendMessageRequest {
            receive_id: api::USER_ID.to_string(),
            msg_type: "text".to_string(),
            content,
            uuid: None,
        },
    )
    .await?;

    Ok(())
}

// 卡片编辑网址 https://open.feishu.cn/tool/cardbuilder
pub async fn send_card_message(
    card_id: &'static str,
    params: HashMap<String, String>,
) -> Result<()> {
    let content = serde_json::to_string(&CardMessage {
        r#type: "template",
        data: CardMessageData {
            template_id: card_id,
            template_variable: params,
        },
    })?;

    api::message::send_message(
        api::message::ReceiveIdType::OpenId,
        SendMessageRequest {
            receive_id: api::USER_ID.to_string(),
            msg_type: "interactive".to_string(),
            content,
            uuid: None,
        },
    )
    .await?;

    Ok(())
}

pub async fn send_card_message_to_chat(
    chat_id: &str,
    card_id: &'static str,
    params: HashMap<String, String>,
) -> Result<()> {
    let content = serde_json::to_string(&CardMessage {
        r#type: "template",
        data: CardMessageData {
            template_id: card_id,
            template_variable: params,
        },
    })?;

    api::message::send_message(
        api::message::ReceiveIdType::ChatId,
        SendMessageRequest {
            receive_id: chat_id.to_string(),
            msg_type: "interactive".to_string(),
            content,
            uuid: None,
        },
    )
    .await?;

    Ok(())
}

#[test]
pub fn test_send_plain_message() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            send_plain_message("test send plain message").await.unwrap();
        })
}

#[test]
pub fn test_send_card_message() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            send_card_message("ctp_AAwmpvPiOumv", HashMap::new())
                .await
                .unwrap();
        })
}

#[test]
pub fn test_send_card_message_with_param() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut param: HashMap<String, String> = HashMap::new();
            param.insert("test".to_string(), "\nthis is a test message".to_string());
            send_card_message("ctp_AAwmpvPiOumv", param).await.unwrap();
        })
}
