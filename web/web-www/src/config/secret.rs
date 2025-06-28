use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SecretConfig {
    pub auth_key: Option<String>,
}

const LOCAL_STORAGE_KEY: &str = "secret";

impl SecretConfig {
    pub fn load_from_localstorage() -> Self {
        LocalStorage::get::<Self>(LOCAL_STORAGE_KEY).unwrap_or_default()
    }

    pub fn save_to_localstorage(&self) {
        _ = LocalStorage::set(LOCAL_STORAGE_KEY, self);
    }
}
