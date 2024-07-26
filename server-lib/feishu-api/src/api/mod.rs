use std::sync::OnceLock;

pub mod auth;
pub mod message;

static USER_ID: OnceLock<&'static str> = OnceLock::new();
const FEISHU_HOST: &str = "https://open.feishu.cn";

pub fn get_user_id() -> &'static str {
    USER_ID.get_or_init(|| std::env::var("RUST_WEB_FEISHU_USER_ID").unwrap().leak())
}
