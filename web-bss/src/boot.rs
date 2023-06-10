use std::sync::Arc;

use crate::handler_api::*;
use crate::handler_ws::*;
use crate::rocket::futures::SinkExt;
use crate::util::http_decode::*;
use crate::ws::WsBiz;
use crate::ws::WsCon;
use crate::ws::WsMsg;
use log::warn;
use tokio::sync::mpsc::channel;
use tokio::sync::mpsc::Sender;
use tokio::{
    io::{AsyncWriteExt, BufStream},
    net::{TcpListener, TcpStream},
};
use tokio_websockets::Message;
use tokio_websockets::ServerBuilder;

pub fn init_websocket() -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
        while let Ok((stream, _)) = listener.accept().await {
            serve_websocket(stream).await;
        }
    })
}

pub fn init_rocket() -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        let routes = routes![home_events];
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

    let (sender, mut receiver) = channel(16);
    let mut biz = match query_websocket_client(&request, Arc::new(sender)) {
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

    tokio::spawn(async move {
        biz.on_open().await;

        loop {
            tokio::select! {
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
                                    let _ = ws_stream.send(Message::pong(&[0u8; 0][0..0])).await;
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
    });
}

fn query_websocket_client(
    request: &HttpRequest,
    sender: Arc<Sender<WsMsg>>,
) -> Option<Box<dyn WsBiz + Send>> {
    for i in 0..WEBSOCKET_BIZ_LIST.len() {
        let factory = &WEBSOCKET_BIZ_LIST[i];
        if let Some(biz) = factory.create_if_match(&request, WsCon::from_sender(sender.clone())) {
            return Some(biz);
        }
    }

    None
}
