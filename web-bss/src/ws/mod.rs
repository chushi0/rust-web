use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc::Sender;

pub mod game;

pub trait WsBizFactory {
    fn create_if_match(
        &self,
        request: &crate::util::http_decode::HttpRequest,
        con: WsCon,
    ) -> Option<Box<dyn WsBiz + Send>>;
}

#[async_trait]
pub trait WsBiz {
    async fn on_open(&mut self) {}

    async fn on_text_message(&mut self, _msg: &str) {}

    async fn on_binary_message(&mut self, _msg: &[u8]) {}

    async fn on_close(&mut self) {}
}

pub struct WsCon {
    sender: Arc<Sender<WsMsg>>,
    close: bool,
}

#[derive(Debug)]
pub enum WsMsg {
    Text(String),
    Binary(Vec<u8>),
    Close,
}

impl WsCon {
    pub fn from_sender(sender: Arc<Sender<WsMsg>>) -> WsCon {
        WsCon {
            sender: sender,
            close: false,
        }
    }

    pub async fn send_text(&self, msg: String) -> Result<()> {
        if !self.close {
            self.sender.send(WsMsg::Text(msg)).await?;
        }
        Ok(())
    }

    pub async fn send_binary(&self, msg: Vec<u8>) -> Result<()> {
        if !self.close {
            self.sender.send(WsMsg::Binary(msg)).await?;
        }
        Ok(())
    }

    pub async fn close(&mut self) -> Result<()> {
        if !self.close {
            self.sender.send(WsMsg::Close).await?;
            self.close = true;
        }
        Ok(())
    }
}
