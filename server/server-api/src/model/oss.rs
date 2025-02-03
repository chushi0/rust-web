use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct GetUploadSignatureRequest {
    pub content_type: String,
}

#[derive(Debug, Serialize)]
pub struct GetUploadSignatureResponse {
    pub uri: String,
    pub url: String,
    pub signature: String,
    pub extra_headers: Vec<ExtraHeader>,
}

#[derive(Debug, Serialize)]
pub struct ExtraHeader {
    pub key: String,
    pub value: String,
}
