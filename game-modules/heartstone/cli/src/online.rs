#[derive(Debug)]
pub struct Client {
    pub ws_server_ip: String,
    pub http_server_ip: String,
    pub account: String,
    pub password: String,
    pub room_id: i32,
}

#[async_trait::async_trait]
impl crate::Client for Client {
    async fn run(&self) {
        todo!()
    }
}
