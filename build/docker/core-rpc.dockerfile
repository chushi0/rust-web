FROM rust:latest as build

# prepare compile environment
## install protoc
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*
## setup proxy
ENV RUSTUP_DIST_SERVER https://rsproxy.cn
ENV RUSTUP_UPDATE_ROOT https://rsproxy.cn/rustup
COPY ./build/docker/cargo.toml ${CARGO_HOME}/config.toml

# compile
## prepare environment
WORKDIR /usr/src/rust-web
## copy files and build
COPY . .
RUN cargo build -p server-core-rpc --release

# run environment
FROM debian:bookworm-slim
WORKDIR /usr/local/home
COPY --from=build /usr/src/rust-web/target/release/server-core-rpc ./server-core-rpc
RUN mkdir -p /var/log
CMD ["./server-core-rpc"]