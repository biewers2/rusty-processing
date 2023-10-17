use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use temporal_sdk::ActContext;
use crate::s3_client;

use crate::util::parse_s3_uri;

/// Input to the `download` activity.
/// 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInput {
    /// The S3 URI to download the file from.
    /// 
    pub s3_uri: String,
    
    /// The local path to where the file should be downloaded to.
    /// 
    pub path: PathBuf,
}

/// Activity for downloading a file from S3.
/// 
pub async fn download(_ctx: ActContext, input: DownloadInput) -> anyhow::Result<()> {
    let (bucket, key) = parse_s3_uri(input.s3_uri)?;
    let object = s3_client().await
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    let mut file = tokio::fs::File::create(&input.path).await?;
    let mut body = object.body.into_async_read();
    tokio::io::copy(&mut body, &mut file).await?;
    Ok(())
}