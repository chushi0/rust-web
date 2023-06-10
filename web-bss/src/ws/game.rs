use super::WsBiz;

pub struct GameBiz {
    con: super::WsCon,
}

impl GameBiz {
    pub fn create(con: super::WsCon) -> GameBiz {
        GameBiz { con: con }
    }
}

#[async_trait]
impl WsBiz for GameBiz {
    async fn on_open(&mut self) {
        self.con.send_text("hello".to_string()).await;
    }

    async fn on_binary_message(&mut self, msg: &[u8]) {
        self.con.send_binary(Vec::from(msg)).await;
    }

    async fn on_close(&mut self) {
        info!("websocket close");
    }
}
