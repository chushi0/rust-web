use crate::model::Model;
use crate::rocket::futures::SinkExt;
use log::warn;
use rocket::serde::json::Json;
use tokio::{
    io::{AsyncWriteExt, BufStream},
    net::{TcpListener, TcpStream},
};
use tokio_websockets::ServerBuilder;

#[macro_use]
extern crate rocket;

mod biz;
pub(crate) mod model;
pub(crate) mod service;
pub mod util;

#[get("/home/events")]
async fn home_events() -> Json<Model<biz::home::GetEventsResp>> {
    Json(biz::home::get_events().await.unwrap_or_else(|e| {
        log::error!("handle error: {e}");
        Model::new_error()
    }))
}

#[tokio::main]
async fn main() {
    let ws = init_websocket();
    let api = init_rocket();

    tokio::select! {
        _ = ws => {
            info!("websocket stream stop");
            std::process::exit(0);
        },
        _ = api => {
            info!("api stream stop");
            std::process::exit(0);
        },
    }
}

fn init_websocket() -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
        while let Ok((stream, _)) = listener.accept().await {
            serve_websocket(stream).await;
        }
    })
}

fn init_rocket() -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        let routes = routes![home_events];
        rocket::build()
            .mount("/api/", routes)
            .launch()
            .await
            .unwrap();
    })
}

async fn serve_websocket(stream: TcpStream) {
    let mut buf_stream = BufStream::new(stream);
    let request = match util::http_decode::parse_http_request(&mut buf_stream).await {
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
    match util::http_decode::websocket_upgrade_handshake(&mut buf_stream, &request).await {
        Ok(v) => v,
        Err(e) => {
            warn!("websocket handshake error: {e}");
            let _ = buf_stream
                .write(b"HTTP/1.1 400 BadRequest\r\nConnection: reset\r\n\r\n")
                .await;
            let _ = buf_stream.flush().await;
            return;
        }
    }
    let mut ws_stream = ServerBuilder::new().serve(buf_stream);

    tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_stream.next().await {
            if msg.is_text() || msg.is_binary() {
                let _ = ws_stream.send(msg).await;
            }
        }
    });
}
