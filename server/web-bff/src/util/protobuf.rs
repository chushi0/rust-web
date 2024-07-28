use anyhow::Result;
use idl_gen::bff_websocket_client::BoxProtobufPayload;
use protobuf::Message;

pub fn pack_message<T>(msg: T) -> Result<Vec<u8>>
where
    T: protobuf::Message,
{
    let payload = BoxProtobufPayload {
        name: T::NAME.to_string(),
        payload: msg.write_to_bytes()?,
        ..Default::default()
    };

    Ok(payload.write_to_bytes()?)
}
