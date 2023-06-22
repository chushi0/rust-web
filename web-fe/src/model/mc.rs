use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AdvancementConfig {
    pub id: String,
    pub title: String,
    pub description: String,
    pub icon: Option<String>,
    pub frame: String,
    pub parent: Option<String>,
    pub requirements: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AdvancementData {
    pub id: String,
    pub done_criteria: Vec<String>,
    pub done: bool,
}
