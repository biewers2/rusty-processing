use std::os::unix::fs::MetadataExt;

use aws_sdk_s3::primitives::ByteStream;
use bytesize::MB;
use tokio::io::AsyncReadExt;

use crate::io::multipart_uploader::MultipartUploader;
use crate::services::s3_client;
use crate::util::parse_s3_uri;

pub async fn upload(mut file: tokio::fs::File, output_s3_uri: String) -> anyhow::Result<()> {
    if file.metadata().await?.size() > MB * 10 {
        let uploader = MultipartUploader::new(output_s3_uri)?;
        uploader.upload(&mut file).await
    } else {
        upload_file(file, output_s3_uri).await
    }
}

async fn upload_file(
    mut file: tokio::fs::File,
    output_s3_uri: String,
) -> anyhow::Result<()> {
    let (bucket, key) = parse_s3_uri(output_s3_uri.as_str())?;

    let mut buf = vec![];
    file.read_to_end(&mut buf).await?;

    s3_client().await
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(ByteStream::from(buf))
        .send()
        .await?;

    Ok(())
}
