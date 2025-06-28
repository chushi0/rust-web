use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode},
    response::Response,
    routing::{future::RouteFuture, Route},
};
use futures::ready;
use tower_layer::Layer;
use tower_service::Service;

const NO_CACHE: HeaderValue = HeaderValue::from_static("max-age=0, no-cache, no-store");
const CACHE_IMMUTABLE: HeaderValue = HeaderValue::from_static("max-age=31536000, immutable");
const CACHE_MUTABLE: HeaderValue = HeaderValue::from_static("max-age=0, must-revalidate");

#[derive(Clone, Copy)]
pub struct WebCache;
#[derive(Clone)]
pub struct WebCacheService(Route);
pub struct WebCacheFuture(RouteFuture<Infallible>);

impl Layer<Route> for WebCache {
    type Service = WebCacheService;

    fn layer(&self, inner: Route) -> Self::Service {
        WebCacheService(inner)
    }
}

impl Service<Request> for WebCacheService {
    type Response = Response;
    type Error = Infallible;
    type Future = WebCacheFuture;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        WebCacheFuture(self.0.call(req))
    }
}

impl Future for WebCacheFuture {
    type Output = Result<Response, Infallible>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut response =
            ready!(Pin::new(&mut self.0).poll(cx)).expect("infallible should never be failed");

        if response.status() == StatusCode::OK {
            let headers = response.headers_mut();
            let media_type = headers
                .get("Content-Type")
                .and_then(|header| header.to_str().ok());

            match media_type {
                // *.html (index.html)
                Some("text/html") => {
                    headers.insert("Cache-Control", NO_CACHE);
                }

                // *.css, *.js, *.wasm
                Some("text/css") | Some("text/javascript") | Some("application/wasm") => {
                    headers.insert("Cache-Control", CACHE_IMMUTABLE);
                }

                // other
                _ => {
                    headers.insert("Cache-Control", CACHE_MUTABLE);
                }
            }
        }

        Poll::Ready(Ok(response))
    }
}
