#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

#[allow(deprecated)] // volo::include_service!改成volo::include!和include!都报错，暂时压制警告
mod gen {
    volo::include_service!("volo_gen.rs");
}

mod protos_gen {
    include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
}

pub use gen::volo_gen::*;
pub use protos_gen::*;

#[test]
fn encode() {
    use protobuf::Message;
    static CHARS: [char; 16] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
    ];

    let mut req = protos_gen::bss_websocket_client::ClientLoginRequest::new();
    req.account = "4dbe4f75-5951-4f5c-871c".to_string();
    req.password = "90a4f623d167".to_string();
    let mut pack = protos_gen::bss_websocket_client::BoxProtobufPayload::new();
    pack.name = protos_gen::bss_websocket_client::ClientLoginRequest::NAME.to_string();
    pack.payload = req.write_to_bytes().unwrap();
    let msg = pack.write_to_bytes().unwrap();
    let mut buf = String::new();
    for byte in msg {
        buf.push(CHARS[(byte / 16) as usize]);
        buf.push(CHARS[(byte % 16) as usize]);
    }
    println!("login {buf}");

    let mut req = protos_gen::bss_websocket_client::CreateRoomRequest::new();
    req.game_type = gen::volo_gen::game_backend::GameType::Hearthstone.into();
    req.init_public = false;
    let mut pack = protos_gen::bss_websocket_client::BoxProtobufPayload::new();
    pack.name = protos_gen::bss_websocket_client::CreateRoomRequest::NAME.to_string();
    pack.payload = req.write_to_bytes().unwrap();
    let msg = pack.write_to_bytes().unwrap();
    let mut buf = String::new();
    for byte in msg {
        buf.push(CHARS[(byte / 16) as usize]);
        buf.push(CHARS[(byte % 16) as usize]);
    }
    println!("create_room {buf}");
}
