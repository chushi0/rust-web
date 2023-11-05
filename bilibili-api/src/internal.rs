use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};

use crate::Client;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Model<Data> {
    pub code: i32,
    pub message: String,
    pub result: Data,
}

impl Client {
    pub(crate) fn request_with_auth(&self, request: RequestBuilder) -> RequestBuilder {
        request.header(
            "Cookie",
            format!(
                "DedeUserID={}; DedeUserID__ckMd5={}; SESSDATA={}; bili_jct={}",
                self.dede_user_id, self.dede_user_id_ckmd5, self.sessdata, self.bili_jct
            ),
        )
    }
}
