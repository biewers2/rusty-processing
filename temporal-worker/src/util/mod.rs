use url::{ParseError, Url};

pub fn parse_s3_uri(s3_uri_str: impl AsRef<str>) -> anyhow::Result<(String, String)> {
    let source_url = Url::parse(s3_uri_str.as_ref())?;
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

