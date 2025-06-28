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

WORKDIR /usr/src/rust-web
COPY . .
RUN cargo build -p server-gateway --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    openssl \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/home
COPY --from=build /usr/src/rust-web/target/release/server-gateway ./server-gateway
CMD ["./server-gateway"]