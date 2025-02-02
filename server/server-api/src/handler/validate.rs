use std::sync::OnceLock;

use regex::Regex;

pub fn validate_upload_oss_uri(uri: &str) -> bool {
    const VALIDATE_PATTERN: &str = r#"^/rust-web/upload/[a-zA-Z0-9\-_.]+$"#;
    static VALIDATE_REGEX: OnceLock<Regex> = OnceLock::new();

    let regex = VALIDATE_REGEX
        .get_or_init(|| Regex::new(VALIDATE_PATTERN).expect("failed to parse valid regex"));

    regex.is_match(uri)
}
