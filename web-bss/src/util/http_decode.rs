use anyhow::{anyhow, Result};
use base64::Engine;
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufStream};

#[derive(Debug)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, Vec<String>>,
}

pub async fn parse_http_request<S: AsyncRead + AsyncWrite + Unpin>(
    s: &mut BufStream<S>,
) -> Result<HttpRequest> {
    let mut request = HttpRequest {
        method: String::new(),
        path: String::new(),
        headers: HashMap::new(),
    };

    let mut line = String::new();

    // read request line
    s.read_line(&mut line).await?;
    let parts: Vec<&str> = line.split(' ').collect();
    if parts.len() != 3 && parts.len() != 2 {
        return Err(anyhow!("parse error"));
    }
    request.method = parts[0].to_string();
    request.path = parts[1].to_string();

    // read headers
    line.clear();
    s.read_line(&mut line).await?;
    while line != "\r\n" {
        if !line.ends_with("\r\n") {
            return Err(anyhow!("bad request"));
        }
        let split_index = line.find(": ").ok_or(anyhow!("parse header error"))?;
        let key = &line[0..split_index];
        let val = &line[split_index + 2..line.len() - 2];
        if !request.headers.contains_key(key) {
            request.headers.insert(key.to_string(), Vec::new());
        }
        request
            .headers
            .get_mut(key)
            .expect("insert before")
            .push(val.to_string());
        line.clear();
        s.read_line(&mut line).await?;
    }

    // read content
    let content_length = request.headers.get("Content-Length");
    if let Some(content_length) = content_length {
        if !content_length.is_empty() {
            let mut size: usize = content_length[0].parse()?;
            while size > 0 {
                let buf = s.fill_buf().await?;
                let mut consume_size = buf.len();
                if consume_size > size {
                    consume_size = size;
                }
                s.consume(consume_size);
                size -= consume_size;
            }
            return Ok(request);
        }
    }
    while line != "\r\n" {
        s.read_line(&mut line).await?;
    }

    Ok(request)
}

pub async fn websocket_upgrade_handshake<S: AsyncRead + AsyncWrite + Unpin>(
    s: &mut BufStream<S>,
    req: &HttpRequest,
) -> Result<()> {
    let upgrade = req.headers.get("Upgrade").ok_or(anyhow!("upgrade"))?;
    if upgrade.is_empty() || upgrade[0] != "websocket" {
        return Err(anyhow!("upgrade"));
    }

    let connection = req.headers.get("Connection").ok_or(anyhow!("connection"))?;
    if connection.is_empty() || connection[0] != "Upgrade" {
        return Err(anyhow!("connection"));
    }

    let sec_websocket_version = req
        .headers
        .get("Sec-WebSocket-Version")
        .ok_or(anyhow!("sec-websocket-version"))?;
    if sec_websocket_version.is_empty() || sec_websocket_version[0] != "13" {
        return Err(anyhow!("unsupported version"));
    }

    let sec_websocket_key = req
        .headers
        .get("Sec-WebSocket-Key")
        .ok_or(anyhow!("sec-websocket-key"))?;
    let sec_websocket_key = if sec_websocket_key.is_empty() {
        return Err(anyhow!("sec-websocket-key"));
    } else {
        sec_websocket_key[0].clone()
    };

    let mut sha1 = Sha1::new();
    let mut sha1_out = [0u8; 20];
    sha1.input(sec_websocket_key.as_bytes());
    sha1.input("258EAFA5-E914-47DA-95CA-C5AB0DC85B11".as_bytes());
    sha1.result(&mut sha1_out);

    let mut base64 = String::new();
    base64::engine::general_purpose::STANDARD.encode_string(sha1_out, &mut base64);

    s.write_all(b"HTTP/1.1 101 Switching Protocols\r\nSec-WebSocket-Version: 13\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ")
        .await?;
    s.write_all(base64.as_bytes()).await?;
    s.write_all(b"\r\n\r\n").await?;
    s.flush().await?;

    Ok(())
}
