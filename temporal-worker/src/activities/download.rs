use std::path::Path;
use tempfile::{NamedTempFile, TempPath};

use crate::services::s3_client;
use crate::util::parse_s3_uri;

/// TODO | Make this an activity part of an overall "Process Rusty File" Workflow once
/// TODO | the Temporal Rust SDK stabilizes workflow definitions.
/// 
/// Activity for downloading a file from S3.
/// 
/// This activity downloads a file from S3 and returns a temporary path to the
/// downloaded file.
/// 
pub async fn download(s3_uri: impl AsRef<Path>) -> anyhow::Result<TempPath> {
    let (bucket, key) = parse_s3_uri(s3_uri)?;
    let object = s3_client().await
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    let path = NamedTempFile::new()?.into_temp_path();
    let mut file = tokio::fs::File::create(&path).await?;
    let mut body = object.body.into_async_read();
    tokio::io::copy(&mut body, &mut file).await?;

    Ok(path)
}