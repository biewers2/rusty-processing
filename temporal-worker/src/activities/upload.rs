use std::path::Path;

use aws_sdk_s3::primitives::ByteStream;
use log::error;
use serde::{Deserialize, Serialize};
use tap::Tap;
use temporal_sdk::ActContext;
use tokio::io::AsyncReadExt;

use crate::s3_client;
use crate::util::parse_s3_uri;

/// Input to the `upload` activity.
/// 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadInput {
    /// The local path to the file to upload.
    /// 
    pub path: String,
    
    /// The S3 URI to upload the file to.
    /// 
    pub s3_uri: String,
}

/// Activity for uploading a file to S3.
///
pub async fn upload(_ctx: ActContext, input: UploadInput) -> anyhow::Result<()> {
    let file = tokio::fs::File::open(&input.path).await?;

    // if file.metadata().await?.size() > MB * 10 {
    //     let uploader = MultipartUploader::new(&input.s3_uri)?;
    //     uploader.upload(&mut file).await
    // } else {
        upload_file(file, &input.s3_uri).await
    // }
}

async fn upload_file(
    mut file: tokio::fs::File,
    output_s3_uri: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let (bucket, key) = parse_s3_uri(output_s3_uri)?;

    let mut buf = vec![];
    file.read_to_end(&mut buf).await?;

    s3_client().await
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(ByteStream::from(buf))
        .send()
        .await
        .tap(|result| {
            if let Err(e) = result {
                error!("Error uploading file to S3: {}", e);
            }
        })?;

    Ok(())
}
