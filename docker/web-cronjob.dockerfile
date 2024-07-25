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
RUN cargo build -p web-cronjob --release

# run environment
FROM debian:bullseye-slim
RUN apt-get update
RUN apt-get install -y cron

WORKDIR /usr/local/home
COPY --from=build /usr/src/rust-web/target/release/web-cronjob ./web-cronjob
COPY --from=build /usr/src/rust-web/conf/log4rs.cronjob.yaml ./conf/log4rs.cronjob.yaml
COPY --from=build /usr/src/rust-web/conf/cronjob.crontab ./conf/cronjob.crontab
RUN cat ./conf/cronjob.crontab | crontab
CMD ["cron", "-f"]