use std::collections::HashMap;

use crate::api::{
    self,
    message::{SendMessageRequest, SendMessageToHookRequest},
};
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

pub async fn send_plain_message(content: &str) -> Result<()> {
    let content = serde_json::to_string(&PlainMessage { text: content })?;

    api::message::send_message(
        api::message::ReceiveIdType::OpenId,
        SendMessageRequest {
            receive_id: api::get_user_id(),
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
            receive_id: api::get_user_id(),
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
    card_id: &str,
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
            receive_id: chat_id,
            msg_type: "interactive".to_string(),
            content,
            uuid: None,
        },
    )
    .await?;

    Ok(())
}

pub async fn send_card_message_to_hook(
    hook_id: &str,
    card_template: &str,
    params: HashMap<String, String>,
) -> Result<()> {
    let mut card = card_template.to_string();
    for (k, v) in params {
        card = card.replace(&format!("${{{}}}", k), &v);
    }

    api::message::send_message_to_webhook(
        hook_id,
        SendMessageToHookRequest {
            msg_type: "interactive".to_string(),
            card: serde_json::from_str(&card)?,
        },
    )
    .await?;

    Ok(())
}
