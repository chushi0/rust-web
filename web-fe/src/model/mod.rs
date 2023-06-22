use serde::Deserialize;

pub mod home;
pub mod mc;

#[derive(Debug, Clone, Deserialize)]
pub struct Model<R> {
    pub code: i32,
    pub msg: String,
    pub data: Option<R>,
}
