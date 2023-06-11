use anyhow::Result;
use idl_gen::bss_websocket_client::BoxProtobufPayload;
use protobuf::Message;

pub fn pack_message<T>(msg: T) -> Result<Vec<u8>>
where
    T: protobuf::Message,
{
    let mut payload = BoxProtobufPayload::default();
    payload.name = T::NAME.to_string();
    payload.payload = msg.write_to_bytes()?;

    Ok(payload.write_to_bytes()?)
}
