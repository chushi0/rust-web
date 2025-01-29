use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct GetHomeEventResponse {
    pub events: Vec<DisplayEvent>,
}

#[derive(Debug, Serialize)]
pub struct DisplayEvent {
    pub title: String,
    pub msg: String,
    pub time: i64,
    pub link: String,
}
