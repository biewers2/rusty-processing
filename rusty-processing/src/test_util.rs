use std::io::Read;
use std::path;

use bytes::Bytes;
use rand::Rng;
use tokio::io::AsyncReadExt;

use crate::common::ByteStream;

pub fn read_contents(path: &str) -> anyhow::Result<Vec<u8>> {
    let mut content = vec![];
    std::fs::File::open(path::PathBuf::from(path))?.read_to_end(&mut content)?;
    Ok(content)
}

pub fn byte_stream_from_string(value: impl Into<String>) -> ByteStream {
    let bytes = Bytes::from(value.into());
    Box::pin(async_stream::stream! { yield bytes })
}

pub async fn byte_stream_from_fs(path: path::PathBuf) -> anyhow::Result<ByteStream> {
    let file = tokio::fs::File::open(path).await.unwrap();
    let mut reader = tokio::io::BufReader::new(file);

    let mut buf = vec![];
    reader.read_to_end(&mut buf).await?;
    let bytes = Bytes::from(buf);
    let stream = Box::pin(async_stream::stream! { yield bytes });

    Ok(stream)
}

pub fn random_byte_stream(len: usize) -> ByteStream {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..len).map(|_| rng.gen()).collect();
    let bytes = Bytes::from(bytes);
    Box::pin(async_stream::stream! { yield bytes })
}