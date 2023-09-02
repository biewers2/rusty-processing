use std::{fs, path};
use std::fs::File;
use std::io::Write;

use serde::{Deserialize, Serialize};
use temporal_sdk::ActContext;
use tokio_stream::StreamExt;

use crate::util::parse_s3_uri::parse_s3_uri;
use crate::util::services::s3_client;

#[derive(Deserialize, Debug)]
pub struct DownloadInput {
    pub source_s3_uri: String,
    pub output_file_path: path::PathBuf,
}

#[derive(Serialize, Debug)]
pub struct DownloadOutput {
    pub bytes: usize,
}

pub async fn download(_ctx: ActContext, input: DownloadInput) -> anyhow::Result<DownloadOutput> {
    let (bucket, key) = parse_s3_uri(input.source_s3_uri.as_str())?;
    let object = s3_client().await
        .get_object()
        .bucket(bucket)
        .key(key)
        .send();

    if !input.output_file_path.exists() {
        if let Some(parent) = input.output_file_path.parent() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut file = File::create(input.output_file_path)?;

    let mut byte_count = 0_usize;
    let mut body = object.await?.body;
    while let Some(buf) = body.try_next().await? {
        let count = file.write(&buf)?;
        byte_count += count;
    }

    Ok(DownloadOutput { bytes: byte_count })
}
