use std::io::{BufReader, Read};
use std::path;

use bytes::Bytes;
use tokio::sync::mpsc::Sender;

pub use parse_s3_uri::*;

mod parse_s3_uri;

pub async fn read_to_stream(source: impl Read, sink: Sender<Bytes>) -> anyhow::Result<()> {
    let mut reader = BufReader::new(source);
    let mut buf = [0; 8192];
    loop {
        let bytes_read = reader.read(&mut buf)?;
        if bytes_read == 0 {
            break;
        }
        let bytes = Bytes::copy_from_slice(&buf[..bytes_read]);
        sink.send(bytes).await?;
    }

    Ok(())
}

pub fn path_file_name_or_random(path: impl AsRef<path::Path>) -> String {
    path.as_ref().file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}