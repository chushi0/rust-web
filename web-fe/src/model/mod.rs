use serde::Deserialize;

pub mod home;

#[derive(Debug, Clone, Deserialize)]
pub struct Model<R> {
    #[allow(unused)]
    pub code: i32,
    // #[allow(unused)]
    // pub msg: String,
    #[serde(flatten)]
    pub data: Option<R>,
}
