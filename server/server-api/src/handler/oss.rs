use axum::{extract::Query, Extension};
use server_common::external_api::aliyun::oss::OssClient;

use crate::{
    extract::{error::AppError, response::BodyResponse},
    model::oss::{ExtraHeader, GetUploadSignatureRequest, GetUploadSignatureResponse},
};

#[axum::debug_handler]
pub async fn get_upload_signature(
    Extension(oss_client): Extension<OssClient>,
    Query(req): Query<GetUploadSignatureRequest>,
) -> Result<BodyResponse<GetUploadSignatureResponse>, AppError> {
    let (uri, signature) = oss_client.upload_signature(Some(req.content_type));
    let url = oss_client.path_url(&uri);

    Ok(BodyResponse::new(GetUploadSignatureResponse {
        uri,
        url,
        signature: signature.signature,
        extra_headers: signature
            .extra_params
            .into_iter()
            .map(|(key, value)| ExtraHeader { key, value })
            .collect(),
    }))
}
