use serde::Deserialize;

pub mod home;

#[derive(Debug, Clone, Deserialize)]
pub struct Model<R> {
    pub code: i32,
    pub msg: String,
    pub data: Option<R>,
}
