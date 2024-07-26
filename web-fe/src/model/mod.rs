use serde::Deserialize;

pub mod home;
pub mod mc;

#[derive(Debug, Clone, Deserialize)]
pub struct Model<R> {
    #[allow(unused)]
    pub code: i32,
    #[allow(unused)]
    pub msg: String,
    pub data: Option<R>,
}
