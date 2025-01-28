use std::rc::Rc;

use serde::{Deserialize, Serialize};

// not implements all
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: i64,
    pub node_id: Rc<String>,
    pub name: Rc<String>,
    pub full_name: Rc<String>,
    pub private: bool,
    pub html_url: Rc<String>,
    pub description: Rc<String>,
    pub fork: bool,
}
