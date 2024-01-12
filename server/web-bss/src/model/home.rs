use rocket::serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct EventData {
    pub title: String,
    pub msg: String,
    pub time: i64,
    pub link: String,
}
