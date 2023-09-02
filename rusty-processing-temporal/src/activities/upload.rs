use std::{fs, path};
use std::fs::File;

use aws_sdk_s3::primitives::ByteStream;
use byte_unit::n_mb_bytes;
use filesize::PathExt;
use serde::Deserialize;
use temporal_sdk::ActContext;

use crate::activities::multipart_uploader::MultipartUploader;
use crate::util::parse_s3_uri::parse_s3_uri;
use crate::util::services::s3_client;

const MB_10: u64 = n_mb_bytes!(10) as u64;

#[derive(Deserialize, Debug)]
pub struct UploadInput {
    pub source_file_path: path::PathBuf,
    pub mimetype: Option<String>,
    pub output_s3_uri: String,
}

pub async fn upload(_ctx: ActContext, input: UploadInput) -> anyhow::Result<()> {
    let source_path = input.source_file_path;
    if source_path.is_dir() {
        for path in recurse_dir(source_path.clone())? {
            let path_suffix = path.strip_prefix(&source_path)?;
            let s3_uri = format!("{}/{}", input.output_s3_uri, path_suffix.to_str().unwrap_or(""));
            upload_file(path, &input.mimetype, s3_uri).await?;
        }
        return Ok(())
    } else if source_path.size_on_disk()? > MB_10 {
        let uploader = MultipartUploader::new(input.output_s3_uri)?;
        let mut file = File::open(source_path)?;
        uploader.upload(&mut file).await
    } else {
        upload_file(source_path, &input.mimetype, input.output_s3_uri).await?;
        Ok(())
    }
}

fn recurse_dir(dir: path::PathBuf) -> anyhow::Result<Vec<path::PathBuf>> {
    let mut paths = vec![];
    let mut dirs = vec![dir];

    while let Some(dir) = dirs.pop() {
        let read_dir = fs::read_dir(dir)?;
        for entry_res in read_dir {
            if let Ok(entry) = entry_res {
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else {
                    paths.push(path);
                }
            }
        }
    }

    Ok(paths)
}

async fn upload_file(source_file_path: path::PathBuf, mimetype: &Option<String>, output_s3_uri: String) -> anyhow::Result<()> {
    let (bucket, key) = parse_s3_uri(output_s3_uri.as_str())?;
    let buf = fs::read(source_file_path)?;

    let mut builder = s3_client().await
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(ByteStream::from(buf));

    if let Some(mimetype) = mimetype {
        builder = builder.content_type(mimetype)
    }

    builder.send().await?;
    Ok(())
}