use std::path::Path;
use std::str::FromStr;

use anyhow::anyhow;
use url::{ParseError, Url};

pub fn parse_s3_uri(s3_uri_str: impl AsRef<Path>) -> anyhow::Result<(String, String)> {
    let s3_uri_str = s3_uri_str.as_ref().to_string_lossy().to_string();
    let source_url = Url::from_str(s3_uri_str.as_str())
        .map_err(|_| anyhow!("Failed to parse S3 URL"))?;

    if let (Some(bucket), key) = (source_url.host(), source_url.path()) {
        let key = if let Some(stripped) = key.strip_prefix('/') {
            stripped
        } else {
            key
        };

        Ok((bucket.to_string(), key.to_string()))
    } else {
        Err(ParseError::EmptyHost)?
    }
}

