use rocket::serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AdvancementData {
    pub id: String,
    pub done_criteria: Vec<String>,
    pub done: bool,
}
