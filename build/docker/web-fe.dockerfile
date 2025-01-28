## initialize toolchain
FROM rust:latest
## setup proxy
ENV RUSTUP_DIST_SERVER https://rsproxy.cn
ENV RUSTUP_UPDATE_ROOT https://rsproxy.cn/rustup
# COPY ./docker/cargo.toml ${CARGO_HOME}/config.toml
## install rust toolchain
RUN rustup toolchain add stable-x86_64-unknown-linux-gnu
RUN rustup component add rustfmt --toolchain stable-x86_64-unknown-linux-gnu
RUN cargo install trunk
RUN rustup target add wasm32-unknown-unknown

## code
WORKDIR /usr/src/rust-web
COPY . .

## compile
WORKDIR /usr/src/rust-web/web-fe
RUN trunk build --release

# run environment
# FROM nginx:slim