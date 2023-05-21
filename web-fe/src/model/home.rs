use serde::Deserialize;
use time::OffsetDateTime;

#[derive(Debug, Clone, Deserialize)]
pub struct EventData {
    pub title: String,
    pub msg: String,
    #[serde(with = "time::serde::timestamp")]
    pub time: OffsetDateTime,
    pub link: String,
}
