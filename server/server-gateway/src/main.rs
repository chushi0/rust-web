use std::{
    collections::HashMap,
    convert::Infallible,
    env,
    pin::Pin,
    sync::{Arc, LazyLock},
    time::Duration,
};

use deadpool::managed::{Manager, Object, Pool, RecycleResult};
use http::{HeaderValue, Request, Response, Uri, header::HOST};
use hyper::{
    body::{Body, Bytes, Incoming},
    client::conn::http2::SendRequest,
    service::service_fn,
};
use hyper_util::rt::{TokioExecutor, TokioIo};
use rustls::{
    ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer},
};
use rustls_pemfile::Item;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
};
use tokio_rustls::TlsAcceptor;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let tls_acceptor = make_tls_acceptor_from_env().unwrap();
    let listener = TcpListener::bind("0.0.0.0:8043").await.unwrap();
    tracing::info!("Gateway listening on https://0.0.0.0:8043");

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let acceptor = tls_acceptor.clone();

        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(tls_stream) => {
                    let protocol = tls_stream.get_ref().1.alpn_protocol();
                    match protocol {
                        Some(b"h2") => {
                            if let Err(err) =
                                hyper::server::conn::http2::Builder::new(TokioExecutor::new())
                                    .serve_connection(
                                        TokioIo::new(tls_stream),
                                        service_fn(proxy_handler),
                                    )
                                    .await
                            {
                                tracing::error!("Connection error: {err:?}");
                            }
                        }
                        Some(b"http/1.1") => {
                            if let Err(err) = hyper::server::conn::http1::Builder::new()
                                .serve_connection(
                                    TokioIo::new(tls_stream),
                                    service_fn(proxy_handler),
                                )
                                .with_upgrades()
                                .await
                            {
                                tracing::error!("Connection error: {err:?}");
                            }
                        }
                        alpn => tracing::warn!("unexpect alpn accept: {alpn:?}"),
                    }
                }
                Err(err) => tracing::error!("Connection error: {err:?}"),
            }
        });
    }
}

enum ProxyBody {
    Incoming(Incoming),
    String(String),
}

impl Body for ProxyBody {
    type Data = Bytes;
    type Error = hyper::Error;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>> {
        match self.get_mut() {
            ProxyBody::Incoming(incoming) => Pin::new(incoming).poll_frame(cx),
            ProxyBody::String(string) => Pin::new(string).poll_frame(cx).map_err(|_| {
                unreachable!(
                    "poll_frame for string is never err since its error_type is infallible"
                )
            }),
        }
    }
}

async fn proxy_handler(mut req: Request<Incoming>) -> Result<Response<ProxyBody>, Infallible> {
    let original_host = req
        .headers()
        // .get(HOST)
        .get("x-host")
        .and_then(|v| v.to_str().ok())
        .or(req.uri().host())
        .unwrap_or("")
        .to_owned();

    let Some(target_host) = resolve_target_host(&original_host) else {
        return Ok(Response::builder()
            .status(502)
            .body(ProxyBody::String("Bad Gateway: Host not found".to_owned()))
            .unwrap());
    };

    let uri = format!(
        "http://{}{}",
        target_host,
        req.uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/")
    );
    let uri: Uri = uri.parse().unwrap();

    *req.uri_mut() = uri.clone();
    req.headers_mut().insert(HOST, target_host.parse().unwrap());

    // 添加自定义头表示原始 Host
    req.headers_mut().insert(
        "x-original-host",
        HeaderValue::from_str(&original_host)
            .unwrap_or_else(|_| HeaderValue::from_static("invalid")),
    );

    let mut sender = loop {
        let Ok(sender) = get_connection(target_host.to_owned()).await else {
            return Ok(Response::builder()
                .status(502)
                .body(ProxyBody::String("Bad Gateway: connect failed".to_owned()))
                .unwrap());
        };
        if sender.is_closed() {
            _ = Object::take(sender);
            continue;
        }

        break sender;
    };

    match sender.send_request(req).await {
        Ok(resp) => Ok(resp.map(ProxyBody::Incoming)),
        Err(err) => {
            _ = Object::take(sender);
            eprintln!("Proxy error: {err:?}");
            Ok(Response::builder()
                .status(502)
                .body(ProxyBody::String("Bad Gateway: Upstream error".to_owned()))
                .unwrap())
        }
    }
}

fn resolve_target_host(host: &str) -> Option<&'static str> {
    match extract_subdomain(host) {
        Some("www") | None => Some("api-service.default.svc.cluster.local:8080"),
        _ => None,
    }
}

fn extract_subdomain(domain: &str) -> Option<&str> {
    let mut dots = domain.match_indices('.');

    let first_dot = dots.next();
    let second_dot = dots.next();

    match (first_dot, second_dot) {
        (Some((i1, _)), Some((_i2, _))) => Some(&domain[..i1]),
        _ => None,
    }
}

/// 从环境变量中加载 PEM 格式证书与私钥
fn make_tls_acceptor_from_env() -> Result<TlsAcceptor, Box<dyn std::error::Error>> {
    let cert_pem = env::var("RUSTWEB_CERT_PEM")?;
    let key_pem = env::var("RUSTWEB_KEY_PEM")?;

    let mut cert_reader = cert_pem.as_bytes();
    let mut key_reader = key_pem.as_bytes();

    let cert = rustls_pemfile::certs(&mut cert_reader)
        .map(|it| it.map(|it| it.to_vec()))
        .collect::<Result<Vec<_>, _>>()?;
    // Check the entire PEM file for the key in case it is not first section
    let mut key_vec: Vec<Vec<u8>> = rustls_pemfile::read_all(&mut key_reader)
        .filter_map(|i| match i.ok()? {
            Item::Sec1Key(key) => Some(key.secret_sec1_der().to_vec()),
            Item::Pkcs1Key(key) => Some(key.secret_pkcs1_der().to_vec()),
            Item::Pkcs8Key(key) => Some(key.secret_pkcs8_der().to_vec()),
            _ => None,
        })
        .collect();

    // Make sure file contains only one key
    if key_vec.len() != 1 {
        panic!("private key format not supported");
    }

    let cert = cert.into_iter().map(CertificateDer::from).collect();
    let key = PrivateKeyDer::try_from(key_vec.pop().expect("we have checked len above"))?;

    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert, key)?;

    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(TlsAcceptor::from(Arc::new(config)))
}

struct ConnectionManager {
    host: String,
}

impl Manager for ConnectionManager {
    type Type = SendRequest<Incoming>;
    type Error = ();

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        let Ok(stream) = TcpStream::connect(&self.host).await else {
            return Err(());
        };
        let Ok((sender, conn)) =
            hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(stream)).await
        else {
            return Err(());
        };

        tokio::spawn(conn);

        Ok(sender)
    }

    async fn recycle(
        &self,
        _obj: &mut Self::Type,
        _metrics: &deadpool::managed::Metrics,
    ) -> RecycleResult<Self::Error> {
        RecycleResult::Ok(())
    }
}

async fn get_connection(host: String) -> Result<Object<ConnectionManager>, ()> {
    static POOLS: LazyLock<RwLock<HashMap<String, Pool<ConnectionManager>>>> =
        LazyLock::new(|| RwLock::new(HashMap::new()));

    let read = POOLS.read().await;
    if let Some(pool) = read.get(&host) {
        return pool.get().await.map_err(|_| ());
    }
    drop(read);

    let mut write = POOLS.write().await;
    if !write.contains_key(&host) {
        write.insert(
            host.clone(),
            Pool::builder(ConnectionManager { host: host.clone() })
                .create_timeout(Some(Duration::from_secs(1)))
                .recycle_timeout(Some(Duration::from_secs(30)))
                .runtime(deadpool::Runtime::Tokio1)
                .build()
                .unwrap(),
        );
    }
    let read = write.downgrade();
    read.get(&host).unwrap().get().await.map_err(|_| ())
}
