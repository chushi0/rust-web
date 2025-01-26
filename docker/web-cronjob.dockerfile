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
RUN cargo build -p web-cronjob --release

# run environment
FROM debian:bookworm-slim
RUN apt-get update
RUN apt-get install -y cron

WORKDIR /usr/local/home
COPY --from=build /usr/src/rust-web/conf/log4rs.prod.yaml ./conf/log4rs.yaml
COPY --from=build /usr/src/rust-web/target/release/web-cronjob ./web-cronjob
# COPY --from=build /usr/src/rust-web/conf/cronjob.crontab ./conf/cronjob.crontab
# RUN cat ./conf/cronjob.crontab | crontab
# RUN mkdir -p /var/log
# CMD ["cron", "-f"]