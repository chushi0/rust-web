FROM rust:latest as build

# prepare compile environment
## setup proxy
ENV RUSTUP_DIST_SERVER https://rsproxy.cn
ENV RUSTUP_UPDATE_ROOT https://rsproxy.cn/rustup
COPY ./docker/cargo.toml ${CARGO_HOME}/config.toml
## install rust toolchain
RUN rustup toolchain add stable-x86_64-unknown-linux-gnu
RUN rustup component add rustfmt --toolchain stable-x86_64-unknown-linux-gnu

# compile
## prepare environment
WORKDIR /usr/src/rust-web
## copy files and build
COPY . .
RUN cargo build -p game-backend --release

# run environment
FROM debian:bullseye-slim
WORKDIR /usr/local/home
COPY --from=build /usr/src/rust-web/target/release/game-backend ./game-backend
COPY --from=build /usr/src/rust-web/conf/log4rs.game-backend.yaml ./conf/log4rs.game-backend.yaml
CMD ["./game-backend"]