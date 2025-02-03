use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GetUploadSignatureResponse {
    pub uri: String,
    pub url: String,
    pub signature: String,
    pub extra_headers: Vec<ExtraHeader>,
}

#[derive(Debug, Deserialize)]
pub struct ExtraHeader {
    pub key: String,
    pub value: String,
}
