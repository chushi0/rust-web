use rocket::serde::Serialize;

pub mod home;
pub mod mc;

#[derive(Debug, Clone, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Model<R> {
    pub code: i32,
    pub msg: String,
    pub data: Option<R>,
}

impl<R> Model<R> {
    pub fn from_success(data: R) -> Model<R> {
        Model {
            code: 0,
            msg: "success".to_string(),
            data: Some(data),
        }
    }

    pub fn new_error() -> Model<R> {
        Model {
            code: 500,
            msg: "internal error".to_string(),
            data: None,
        }
    }
}
