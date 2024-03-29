pub mod auth;
pub mod message;

pub static USER_ID: &str = env!("RUST_WEB_FEISHU_USER_ID");
const FEISHU_HOST: &str = "https://open.feishu.cn";
