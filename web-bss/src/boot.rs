use crate::handler_api::*;
use crate::handler_ws::*;
use crate::rocket::futures::SinkExt;
use crate::util::http_decode::*;
use crate::ws::WsBiz;
use crate::ws::WsCon;
use crate::ws::WsMsg;
use log::warn;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use std::vec;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tokio::{
    io::{AsyncWriteExt, BufStream},
    net::{TcpListener, TcpStream},
};
use tokio_websockets::Message;
use tokio_websockets::ServerBuilder;
use volo_grpc::server::Server;
use volo_grpc::server::ServiceBuilder;

pub fn init_grpc() -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        let addr: SocketAddr = "127.0.0.1:13202".parse().unwrap();
        let addr = volo::net::Address::from(addr);

        Server::new()
            .add_service(
                ServiceBuilder::new(idl_gen::bss_websocket::BssWebsocketServiceServer::new(
                    crate::handler_grpc::S,
                ))
                .build(),
            )
            .run(addr)
            .await
            .unwrap();
    })
}

pub fn init_websocket() -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_websocket(stream));
        }
    })
}

pub fn init_rocket() -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        let routes = routes![
            home_events,
            user_new,
            mc_globaldata_advancement,
            mc_playerdata_advancement
        ];
        rocket::build()
            .mount("/api/", routes)
            .launch()
            .await
            .unwrap();
    })
}

lazy_static::lazy_static! {
    static ref WEBSOCKET_BIZ_LIST: Vec<Box<dyn crate::ws::WsBizFactory + Send + Sync>> =
        vec![Box::new(GameBizFactory {})];
}

async fn serve_websocket(stream: TcpStream) {
    let mut buf_stream = BufStream::new(stream);
    let request = match parse_http_request(&mut buf_stream).await {
        Ok(v) => v,
        Err(e) => {
            warn!("websocket handshake error: {e}");
            let _ = buf_stream
                .write(b"HTTP/1.1 400 BadRequest\r\nConnection: reset\r\n\r\n")
                .await;
            let _ = buf_stream.flush().await;
            return;
        }
    };

    let (sender, mut receiver) = unbounded_channel();
    let ping = Arc::new(RwLock::new(0));

    let mut biz = match query_websocket_client(&request, Arc::new(sender), ping.clone()) {
        Some(v) => v,
        None => {
            warn!("websocket no client: {request:?}");
            let _ = buf_stream
                .write(b"HTTP/1.1 404 Not Found\r\nConnection: reset\r\n\r\n")
                .await;
            let _ = buf_stream.flush().await;
            return;
        }
    };

    match websocket_upgrade_handshake(&mut buf_stream, &request).await {
        Ok(v) => v,
        Err(e) => {
            warn!("websocket handshake error: {e}");
            let _ = buf_stream
                .write(b"HTTP/1.1 400 Bad Request\r\nConnection: reset\r\n\r\n")
                .await;
            let _ = buf_stream.flush().await;
            return;
        }
    }
    let mut ws_stream = ServerBuilder::new().serve(buf_stream);

    biz.on_open().await;
    let mut ping_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::default())
        .as_millis();
    let mut ping_data: u32 = 0;
    let mut recv_response = false;
    let _ = ws_stream
        .send(Message::ping(Vec::from(ping_data.to_be_bytes())))
        .await;
    let mut timeout_count = 0;

    loop {
        tokio::select! {
            _ = sleep(Duration::from_secs(10)) => {
                if !recv_response {
                    warn!("client not response in 10 secs");
                    timeout_count += 1;
                    if timeout_count > 10 {
                        let _ = ws_stream.close(None, None).await;
                    }
                }
                recv_response = false;
                ping_data += 1;
                ping_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::default())
                    .as_millis();
                let _ = ws_stream
                    .send(Message::ping(Vec::from(ping_data.to_be_bytes())))
                    .await;
            },
            Some(msg) = receiver.recv() => {
                match msg {
                    WsMsg::Text(msg) => {
                        if let Err(_) = ws_stream.send(Message::text(msg)).await {
                            break;
                        }
                    },
                    WsMsg::Binary(msg) => {
                        if let Err(_) = ws_stream.send(Message::binary(msg)).await {
                            break;
                        }
                    },
                    WsMsg::Close => {
                        if let Err(_) = ws_stream.close(None, None).await {
                            break;
                        }
                    },
                }
            },
            msg = ws_stream.next() => {
                match msg {
                    Some(msg) => match msg{
                        Ok(msg) => {
                            if msg.is_text() {
                                biz.on_text_message(msg.as_text().expect("should be text")).await;
                            } else if msg.is_binary() {
                                biz.on_binary_message(msg.as_data()).await;
                            } else if msg.is_ping() {
                                let _ = ws_stream.send(Message::pong(msg.as_data().clone())).await;
                            } else if msg.is_pong() {
                                let ping_data = ping_data.to_be_bytes();
                                let recv_data = msg.as_data();
                                if equals(&ping_data, recv_data) && !recv_response {
                                    let cur_time = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap_or(Duration::default())
                                        .as_millis();
                                    let delay = cur_time - ping_time;
                                    *ping.write().await = delay as u64;
                                    recv_response = true;
                                    timeout_count = 0;
                                }
                            }
                        },
                        Err(_) => break,
                    },
                    None => break,
                }
            }
        }
    }

    biz.on_close().await;
}

fn query_websocket_client(
    request: &HttpRequest,
    sender: Arc<UnboundedSender<WsMsg>>,
    ping: Arc<RwLock<u64>>,
) -> Option<Box<dyn WsBiz + Send>> {
    for i in 0..WEBSOCKET_BIZ_LIST.len() {
        let factory = &WEBSOCKET_BIZ_LIST[i];
        if let Some(biz) =
            factory.create_if_match(&request, WsCon::from_sender(sender.clone(), ping.clone()))
        {
            return Some(biz);
        }
    }

    None
}

fn equals(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    for i in 0..a.len() {
        if a[i] != b[i] {
            return false;
        }
    }

    true
}
