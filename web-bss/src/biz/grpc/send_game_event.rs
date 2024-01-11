use anyhow::Result;
use idl_gen::bss_websocket::{SendGameEventRequest, SendGameEventResponse};

pub async fn handle(req: &SendGameEventRequest) -> Result<SendGameEventResponse> {
    todo!()
}
