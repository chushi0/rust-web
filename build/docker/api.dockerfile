FROM rust:1.88 as build

# prepare compile environment
## install protoc
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*
## setup proxy
ENV RUSTUP_DIST_SERVER https://rsproxy.cn
ENV RUSTUP_UPDATE_ROOT https://rsproxy.cn/rustup
COPY ./build/docker/cargo.toml ${CARGO_HOME}/config.toml

## install trunk && wasm32 target && wasm-bindgen-cli
RUN cargo install trunk
RUN rustup target add wasm32-unknown-unknown
RUN cargo install -f wasm-bindgen-cli --version 0.2.100
RUN mkdir -p /root/.cache/trunk/wasm-bindgen-0.2.87 \
    && ln -sf /usr/local/cargo/bin/wasm-bindgen /root/.cache/trunk/wasm-bindgen-0.2.87/wasm-bindgen


# compile
## prepare environment
WORKDIR /usr/src/rust-web
## copy files and build
COPY . .
## build server-api
RUN cargo build -p server-api --release
## build web/www
RUN rustup target add wasm32-unknown-unknown
RUN cd ./web/web-www && trunk build --release --dist dist

# run environment
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    openssl \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/home
COPY --from=build /usr/src/rust-web/target/release/server-api ./server-api
COPY --from=build /usr/src/rust-web/web/web-www/dist ./web
CMD ["./server-api"]