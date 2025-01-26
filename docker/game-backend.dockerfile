FROM rust:latest as build

# prepare compile environment
## setup proxy
ENV RUSTUP_DIST_SERVER https://rsproxy.cn
ENV RUSTUP_UPDATE_ROOT https://rsproxy.cn/rustup
COPY ./docker/cargo.toml ${CARGO_HOME}/config.toml

# compile
## prepare environment
WORKDIR /usr/src/rust-web
## copy files and build
COPY . .
RUN cargo build -p game-backend --release

# run environment
FROM debian:bookworm-slim
WORKDIR /usr/local/home
COPY --from=build /usr/src/rust-web/conf/log4rs.prod.yaml ./conf/log4rs.yaml
COPY --from=build /usr/src/rust-web/target/release/game-backend ./game-backend
RUN mkdir -p /var/log
CMD ["./game-backend"]