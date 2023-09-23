use std::fs::File;
use std::io::Read;
use std::os::unix::fs::MetadataExt;

use aws_sdk_s3::primitives::ByteStream;
use byte_unit::n_mb_bytes;

use crate::io::multipart_uploader::MultipartUploader;
use crate::services::s3_client;
use crate::util::parse_s3_uri;

const MB_10: u64 = n_mb_bytes!(10) as u64;

pub async fn upload(mut file: File, output_s3_uri: String) -> anyhow::Result<()> {
    if file.metadata()?.size() > MB_10 {
        let uploader = MultipartUploader::new(output_s3_uri)?;
        uploader.upload(&mut file).await
    } else {
        upload_file(file, output_s3_uri).await
    }
}

async fn upload_file(
    mut file: File,
    output_s3_uri: String,
) -> anyhow::Result<()> {
    let (bucket, key) = parse_s3_uri(output_s3_uri.as_str())?;

    let mut buf = vec![];
    file.read_to_end(&mut buf)?;

    s3_client().await
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(ByteStream::from(buf))
        .send()
        .await?;

    Ok(())
}
