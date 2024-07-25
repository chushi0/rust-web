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
RUN cargo build -p web-bss --release

# run environment
FROM debian:bullseye-slim
WORKDIR /usr/local/home
COPY --from=build /usr/src/rust-web/target/release/web-bss ./web-bss
COPY --from=build /usr/src/rust-web/conf/Rocket.bss.toml ./conf/Rocket.bss.toml
CMD ["./web-bss"]