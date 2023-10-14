use std::os::unix::fs::MetadataExt;
use std::path::Path;
use aws_sdk_s3::primitives::ByteStream;
use bytesize::MB;
use log::error;
use tap::Tap;
use tokio::io::AsyncReadExt;
use crate::io::MultipartUploader;
use crate::services::s3_client;
use crate::util::parse_s3_uri;

/// TODO | Make this an activity part of an overall "Process Rusty File" Workflow once
/// TODO | the Temporal Rust SDK stabilizes workflow definitions.
///
/// Activity for uploading a file to S3.
///
pub async fn upload(path: impl AsRef<Path>, output_s3_uri: impl AsRef<Path>) -> anyhow::Result<()> {
    let mut file = tokio::fs::File::open(path).await?;

    if file.metadata().await?.size() > MB * 10 {
        let uploader = MultipartUploader::new(output_s3_uri)?;
        uploader.upload(&mut file).await
    } else {
        upload_file(file, output_s3_uri).await
    }
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