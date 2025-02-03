use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit};
use anyhow::{anyhow, Result};
use base64::Engine;
use hkdf::Hkdf;
use serde::{Deserialize, Serialize, Serializer};
use sha2::{digest::generic_array::GenericArray, Sha256};

use crate::config::secret::SecretConfig;

pub mod home;
pub mod mc;
pub mod oss;

#[derive(Debug, Clone, Deserialize)]
pub struct Model<R> {
    #[allow(unused)]
    pub code: i32,
    // #[allow(unused)]
    // pub msg: String,
    #[serde(flatten)]
    pub data: Option<R>,
}

#[derive(Debug, Serialize)]
pub struct EncryptRequest {
    time: i64,
    #[serde(serialize_with = "serialize_bytes")]
    salt: Vec<u8>,
    #[serde(serialize_with = "serialize_bytes")]
    nonce: Vec<u8>,
    #[serde(serialize_with = "serialize_bytes")]
    payload: Vec<u8>,
}

fn serialize_bytes<S: Serializer>(val: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    if serializer.is_human_readable() {
        let base64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(val);
        serializer.serialize_str(&base64)
    } else {
        serializer.serialize_bytes(val)
    }
}

impl EncryptRequest {
    pub fn encrypt_payload(payload: &[u8]) -> Result<Self> {
        let time = js_sys::Date::new_0().get_time() as i64 / 1000;
        let mut salt = vec![0u8; 12];
        let mut nonce = vec![0u8; 12];
        Self::random_fill(&mut salt);
        Self::random_fill(&mut nonce);

        let auth_key = SecretConfig::load_from_localstorage()
            .auth_key
            .unwrap_or_default();
        let hk = Hkdf::<Sha256>::new(Some(&salt), auth_key.as_bytes());
        let mut key = [0u8; 32];
        hk.expand(&time.to_le_bytes(), &mut key)
            .map_err(|e| anyhow!("failed to expand key: {e}"))?;
        let cipher = Aes256Gcm::new_from_slice(&key)?;
        let nonce_ga = GenericArray::from_slice(&nonce);
        let encrypt = cipher
            .encrypt(nonce_ga, payload)
            .map_err(|e| anyhow!("failed to encrypt payload: {e}"))?;

        Ok(Self {
            time,
            salt,
            nonce,
            payload: encrypt,
        })
    }

    pub fn to_query_params(&self) -> [(&'static str, String); 4] {
        [
            ("time", self.time.to_string()),
            (
                "salt",
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&self.salt),
            ),
            (
                "nonce",
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&self.nonce),
            ),
            (
                "payload",
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&self.payload),
            ),
        ]
    }

    fn random_fill(arr: &mut [u8]) {
        arr.iter_mut().for_each(|x| {
            *x = (js_sys::Math::random() * (u8::MAX as f64)) as u8;
        });
    }
}
