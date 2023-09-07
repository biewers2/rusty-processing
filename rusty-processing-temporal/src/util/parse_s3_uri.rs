use url::{ParseError, Url};

pub fn parse_s3_uri(s3_uri_str: &str) -> anyhow::Result<(String, String)> {
    let source_url = Url::parse(s3_uri_str)?;
    if let (Some(bucket), key) = (source_url.host(), source_url.path()) {
        let key = if key.starts_with('/') { &key[1..] } else { key };
        Ok((bucket.to_string(), key.to_string()))
    } else {
        Err(ParseError::EmptyHost)?
    }
}
