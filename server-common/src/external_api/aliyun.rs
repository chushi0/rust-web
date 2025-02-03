pub mod oss {
    use std::{collections::HashMap, str::FromStr};

    use anyhow::{bail, Result};
    use bytes::Bytes;
    use chrono::{DateTime, Duration, Utc};
    use const_format::concatcp;
    use futures::Stream;
    use itertools::Itertools;
    use reqwest::{
        header::{HeaderName, HeaderValue},
        Client as HttpClient,
    };
    use secrecy::{ExposeSecret, SecretString};
    use tracing::info;
    use uuid::Uuid;

    pub const RUSTWEB_PREFIX: &str = "/rust-web/";
    pub const UPLOAD_PREFIX: &str = concatcp!(RUSTWEB_PREFIX, "upload/");

    const ENV_BUCKET_HOST: &str = "RUSTWEB_ALIYUN_OSS_BUCKET_HOST";
    const ENV_BUCKET_NAME: &str = "RUSTWEB_ALIYUN_OSS_BUCKET_NAME";
    const ENV_BUCKET_REGION: &str = "RUSTWEB_ALIYUN_OSS_BUCKET_REGION";
    const ENV_ACCESS_KEY_ID: &str = "RUSTWEB_ALIYUN_OSS_ACCESS_KEY_ID";
    const ENV_ACCESS_KEY_SECRET: &str = "RUSTWEB_ALIYUN_OSS_ACCESS_KEY_SECRET";

    #[derive(Clone)]
    pub struct OssClient {
        bucket_host: String,
        bucket_name: String,
        bucket_region: String,
        access_key_id: SecretString,
        access_key_secret: SecretString,
    }

    pub struct HttpOssClient<'oss, 'http> {
        oss_client: &'oss OssClient,
        http_client: &'http HttpClient,
    }

    pub struct SignResult {
        pub signature: String,
        pub extra_params: HashMap<String, String>,
    }

    impl OssClient {
        pub fn new(
            bucket_host: String,
            bucket_name: String,
            bucket_region: String,
            access_key_id: String,
            access_key_secret: String,
        ) -> Self {
            Self {
                bucket_host,
                bucket_name,
                bucket_region,
                access_key_id: access_key_id.into(),
                access_key_secret: access_key_secret.into(),
            }
        }

        pub fn from_env() -> Result<Self> {
            Ok(Self {
                bucket_host: std::env::var(ENV_BUCKET_HOST)?,
                bucket_name: std::env::var(ENV_BUCKET_NAME)?,
                bucket_region: std::env::var(ENV_BUCKET_REGION)?,
                access_key_id: std::env::var(ENV_ACCESS_KEY_ID)?.into(),
                access_key_secret: std::env::var(ENV_ACCESS_KEY_SECRET)?.into(),
            })
        }

        fn sign_url<'q, 'h, 'ah, Query, Header, AdditionalHeader>(
            &self,
            verb: &str,
            uri: &str,
            query: Query,
            header: Header,
            additional_headers: AdditionalHeader,
            expires: Duration,
        ) -> SignResult
        where
            Query: Iterator<Item = (&'q str, &'q str)> + Sized,
            Header: Iterator<Item = (&'h str, &'h str)> + Sized,
            AdditionalHeader: Iterator<Item = &'ah str> + Sized + Clone,
        {
            let now = Utc::now();

            let request_time = now.format("%Y%m%dT%H%M%SZ");
            let request_day = now.format("%Y%m%d");
            let access_key_id = &self.access_key_id.expose_secret();
            let access_region = &self.bucket_region;
            let credential =
                format!("{access_key_id}/{request_day}/{access_region}/oss/aliyun_v4_request");

            let extra_query = [
                ("x-oss-signature-version", "OSS4-HMAC-SHA256".to_owned()),
                ("x-oss-credential", credential),
                ("x-oss-date", request_time.to_string()),
                ("x-oss-expires", expires.num_seconds().to_string()),
                (
                    "x-oss-additional-headers",
                    additional_headers.clone().join(";"),
                ),
            ]
            .into_iter()
            .filter(|(_, v)| !v.is_empty())
            .collect::<Vec<_>>();

            let signature = self.sign(
                verb,
                uri,
                extra_query
                    .iter()
                    .map(|(k, v)| (*k, v.as_ref()))
                    .chain(query.map(|v| v)),
                header,
                additional_headers,
                now,
            );

            SignResult {
                signature,
                extra_params: extra_query
                    .into_iter()
                    .map(|(k, v)| (k.to_owned(), v))
                    .collect(),
            }
        }

        fn sign_header<'q, 'h, 'ah, Query, Header, AdditionalHeader>(
            &self,
            verb: &str,
            uri: &str,
            query: Query,
            header: Header,
            additional_headers: AdditionalHeader,
        ) -> SignResult
        where
            Query: Iterator<Item = (&'q str, &'q str)> + Sized,
            Header: Iterator<Item = (&'h str, &'h str)> + Sized,
            AdditionalHeader: Iterator<Item = &'ah str> + Sized + Clone,
        {
            let now = Utc::now();

            let request_time = now.format("%Y%m%dT%H%M%SZ");
            let request_day = now.format("%Y%m%d");
            let access_key_id = &self.access_key_id.expose_secret();
            let access_region = &self.bucket_region;
            let credential =
                format!("{access_key_id}/{request_day}/{access_region}/oss/aliyun_v4_request");
            let additional_header_key = additional_headers.clone().join(";");

            let extra_headers = [
                ("x-oss-content-sha256", "UNSIGNED-PAYLOAD".to_owned()),
                ("x-oss-date", request_time.to_string()),
            ];

            let signature = self.sign(
                verb,
                uri,
                query,
                header
                    .map(|v| v)
                    .chain(extra_headers.iter().map(|(k, v)| (*k, v.as_ref()))),
                additional_headers,
                now,
            );

            SignResult {
                signature: if additional_header_key.is_empty() {
                    format!("OSS4-HMAC-SHA256 Credential={credential},Signature={signature}")
                } else {
                    format!("OSS4-HMAC-SHA256 Credential={credential},SignedHeaders={additional_header_key},Signature={signature}")
                },
                extra_params: extra_headers
                    .into_iter()
                    .map(|(k, v)| (k.to_owned(), v))
                    .collect(),
            }
        }

        fn sign<'q, 'h, 'ah, Query, Header, AdditionalHeader>(
            &self,
            verb: &str,
            uri: &str,
            query: Query,
            header: Header,
            additional_headers: AdditionalHeader,
            now: DateTime<Utc>,
        ) -> String
        where
            Query: Iterator<Item = (&'q str, &'q str)> + Sized,
            Header: Iterator<Item = (&'h str, &'h str)> + Sized,
            AdditionalHeader: Iterator<Item = &'ah str> + Sized,
        {
            let canonical_request = {
                let bucket_name = &self.bucket_name;
                let query_string = query
                    .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
                    .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                    .join("&");
                let header_string = header
                    .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
                    .map(|(k, v)| format!("{}:{}\n", k.trim(), v.trim()))
                    .join("");
                let additional_header_string = additional_headers.into_iter().sorted().join("\n");

                format!("{verb}\n/{bucket_name}{uri}\n{query_string}\n{header_string}\n{additional_header_string}\nUNSIGNED-PAYLOAD")
            };
            info!("canonical_request: {}", canonical_request);

            let string_to_sign = {
                let request_time = now.format("%Y%m%dT%H%M%SZ");
                let request_day = now.format("%Y%m%d");
                let region = &self.bucket_region;
                let hash = hmac_sha256::Hash::hash(canonical_request.as_bytes())
                    .into_iter()
                    .map(|n| format!("{:02x}", n))
                    .join("");
                format!("OSS4-HMAC-SHA256\n{request_time}\n{request_day}/{region}/oss/aliyun_v4_request\n{hash}")
            };
            info!("string_to_sign: {}", string_to_sign);

            let signature = {
                let date_key = hmac_sha256::HMAC::mac(
                    now.format("%Y%m%d").to_string(),
                    format!("aliyun_v4{}", self.access_key_secret.expose_secret()),
                );
                let date_region_key = hmac_sha256::HMAC::mac(&self.bucket_region, date_key);
                let date_region_service_key = hmac_sha256::HMAC::mac("oss", date_region_key);
                let signing_key =
                    hmac_sha256::HMAC::mac("aliyun_v4_request", date_region_service_key);

                hmac_sha256::HMAC::mac(string_to_sign, signing_key)
            };

            signature.into_iter().map(|n| format!("{:02x}", n)).join("")
        }

        pub fn path_url(&self, uri: &str) -> String {
            format!("https://{}{}", self.bucket_host, uri)
        }

        pub fn download_url(&self, path: &str, expires: Duration) -> String {
            let sign_result = self.sign_url(
                "GET",
                path,
                [].into_iter(),
                [].into_iter(),
                [].into_iter(),
                expires,
            );

            format!(
                "https://{}{}?{}",
                self.bucket_host,
                path,
                sign_result.to_query_string()
            )
        }

        pub fn upload_signature(&self, content_type: Option<String>) -> (String, SignResult) {
            // random uri
            let uri = {
                let timestamp = Utc::now().timestamp();
                let random = Uuid::new_v4().to_string();

                format!("{}{}-{}", UPLOAD_PREFIX, timestamp, random)
            };

            let sign = self.sign_header(
                "PUT",
                &uri,
                [].into_iter(),
                content_type
                    .as_ref()
                    .map(|v| ("content-type", v.as_ref()))
                    .into_iter(),
                [].into_iter(),
            );

            (uri, sign)
        }

        pub fn with_http<'oss, 'http>(
            &'oss self,
            client: &'http HttpClient,
        ) -> HttpOssClient<'oss, 'http> {
            HttpOssClient {
                oss_client: self,
                http_client: client,
            }
        }
    }

    impl SignResult {
        pub fn to_query_string(&self) -> String {
            self.extra_params
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .chain([format!("x-oss-signature={}", self.signature)].into_iter())
                .join("&")
        }
    }

    impl<'oss, 'http> HttpOssClient<'oss, 'http> {
        pub async fn get_object(
            &self,
            uri: &str,
        ) -> Result<impl Stream<Item = Result<Bytes, reqwest::Error>>> {
            let sign = self.oss_client.download_url(uri, Duration::seconds(30));
            let response = self.http_client.get(sign).send().await?;
            if !response.status().is_success() {
                bail!("get object failed: {}", response.status())
            }

            Ok(response.bytes_stream())
        }

        pub async fn copy_object(&self, src: &str, dst: &str) -> Result<()> {
            let src = format!("/{}{}", self.oss_client.bucket_name, src);
            let header = [("x-oss-copy-source", &src as &str)];

            let sign = self.oss_client.sign_header(
                "PUT",
                dst,
                [].into_iter(),
                header.iter().map(|v| *v),
                [].into_iter(),
            );

            let response = self
                .http_client
                .put(self.oss_client.path_url(dst))
                .headers(
                    header
                        .iter()
                        .map(|v| *v)
                        .chain(
                            sign.extra_params
                                .iter()
                                .map(|(k, v)| (k.as_ref(), v.as_ref())),
                        )
                        .map(|(k, v)| Ok((HeaderName::from_str(k)?, HeaderValue::from_str(v)?)))
                        .collect::<Result<_>>()?,
                )
                .header("Authorization", sign.signature)
                .send()
                .await?;

            if !response.status().is_success() {
                bail!(
                    "copy object failed: {}, {}",
                    response.status(),
                    response.text().await?
                )
            }
            Ok(())
        }

        pub async fn delete_object(&self, uri: &str) -> Result<()> {
            let sign = self.oss_client.sign_header(
                "DELETE",
                uri,
                [].into_iter(),
                [].into_iter(),
                [].into_iter(),
            );

            let response = self
                .http_client
                .delete(self.oss_client.path_url(uri))
                .headers(
                    sign.extra_params
                        .iter()
                        .map(|(k, v)| (k.as_ref(), v.as_ref()))
                        .map(|(k, v)| Ok((HeaderName::from_str(k)?, HeaderValue::from_str(v)?)))
                        .collect::<Result<_>>()?,
                )
                .header("Authorization", sign.signature)
                .send()
                .await?;

            if !response.status().is_success() {
                bail!("delete object failed: {}", response.status())
            }
            Ok(())
        }
    }
}
