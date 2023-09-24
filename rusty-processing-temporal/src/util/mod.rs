use std::ops::{Deref, DerefMut};
use std::path;
use byte_unit::ByteUnit::MB;

use bytes::Bytes;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::ReceiverStream;

pub use parse_s3_uri::*;
use rusty_processing::common::ByteStream;

mod parse_s3_uri;

pub async fn read_to_stream(mut source: Box<dyn AsyncRead + Send + Unpin>) -> anyhow::Result<(ByteStream, JoinHandle<anyhow::Result<()>>)> {
    let (sink, stream) = tokio::sync::mpsc::channel(1000);

    let read_fut = tokio::spawn(async move {
        let mut buf = vec![0; 100000];

        loop {
            let bytes_read = source.read(&mut buf).await?;
            if bytes_read == 0 {
                break;
            }
            println!("Transferring {} bytes", bytes_read);
            let bytes = Bytes::copy_from_slice(&buf[..bytes_read]);
            sink.send(bytes).await?;
        }

        Ok(())
    });

    let stream = Box::new(ReceiverStream::new(stream));
    Ok((stream, read_fut))
}

pub fn path_file_name_or_random(path: impl AsRef<path::Path>) -> String {
    path.as_ref().file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}