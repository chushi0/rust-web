use lazy_static::lazy_static;
use tokio::sync::RwLock;

pub(crate) mod api;
pub mod sdk;

#[derive(Debug)]
pub(crate) struct AuthKey {
    pub(crate) key: String,
    pub(crate) expire_time: u64,
}

lazy_static! {
    pub(crate) static ref AUTH_KEY: RwLock<AuthKey> = RwLock::new(AuthKey::new_invalid());
}

impl AuthKey {
    const fn new_invalid() -> AuthKey {
        AuthKey {
            key: String::new(),
            expire_time: 0,
        }
    }

    fn get_token(&self) -> Option<String> {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("get system time fail")
            .as_secs();
        if time + 30 < self.expire_time {
            Some(self.key.clone())
        } else {
            None
        }
    }

    async fn get_token_with_fetch(&mut self) -> Option<String> {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("get system time fail")
            .as_secs();
        if time + 30 < self.expire_time {
            return Some(self.key.clone());
        }

        let auth_token = api::auth::get_tenant_access_token().await.ok()?;
        self.expire_time = auth_token.expire;
        self.key = auth_token.token;
        Some(self.key.clone())
    }
}

pub(crate) async fn get_token() -> Option<String> {
    let auth_key = AUTH_KEY.read().await;
    if let Some(key) = auth_key.get_token() {
        return Some(key);
    }
    drop(auth_key);
    let mut auth_key = AUTH_KEY.write().await;
    auth_key.get_token_with_fetch().await
}
