//!
//! # Test Utilities
//!
#![warn(missing_docs)]

use std::io::Read;
use std::path;
use std::path::Path;
use std::pin::Pin;
use bytes::Bytes;
use rand::Rng;
use tempfile::{NamedTempFile, TempPath};
use tokio::io;
use tokio::io::AsyncReadExt;
use tokio_stream::Stream;

// Define the same type alias as found in `streaming/src/lib.rs`, as importing that crate here
// would create a circular dependency.
type ByteStream = Pin<Box<dyn Stream<Item=Bytes> + Send + Sync>>;

pub fn read_contents(path: &str) -> Option<Vec<u8>> {
    let mut content = vec![];
    std::fs::File::open(path::PathBuf::from(path))
        .and_then(|mut file| file.read_to_end(&mut content))
        .map(|_| content)
        .ok()
}

pub fn byte_stream_from_string(value: impl Into<String>) -> ByteStream {
    let bytes = Bytes::from(value.into());
    Box::pin(async_stream::stream! { yield bytes })
}

pub async fn byte_stream_from_fs(path: impl AsRef<Path>) -> io::Result<ByteStream> {
    let file = tokio::fs::File::open(path).await?;
    let mut reader = tokio::io::BufReader::new(file);

    let mut buf = vec![];
    reader.read_to_end(&mut buf).await?;
    let bytes = Bytes::from(buf);
    let stream = Box::pin(async_stream::stream! { yield bytes });

    Ok(stream)
}

pub fn random_bytes(len: usize) -> Box<Vec<u8>> {
    let mut rng = rand::thread_rng();
    Box::new((0..len).map(|_| rng.gen()).collect::<Vec<u8>>())
}

pub fn random_byte_stream(len: usize) -> (Bytes, ByteStream) {
    let bytes = Bytes::from(*random_bytes(len));
    (bytes.clone(), Box::pin(async_stream::stream! { yield bytes }))
}

pub fn string_as_byte_stream(value: impl Into<String>) -> ByteStream {
    let bytes = Bytes::from(value.into());
    Box::pin(async_stream::stream! { yield bytes })
}

pub fn temp_path() -> std::io::Result<TempPath> {
    Ok(NamedTempFile::new()?.into_temp_path())
}

#[cfg(test)]
mod tests {
    use tokio_stream::StreamExt;
    use super::*;

    async fn collect_byte_stream(stream: ByteStream) -> Vec<u8> {
        let mut data = vec![];
        let mut stream = stream;
        while let Some(bytes) = stream.next().await {
            data.extend_from_slice(&bytes);
        }
        data
    }

    #[test]
    fn test_read_contents() {
        let contents = read_contents("../resources/jpg/PA280041.JPG");
        assert!(contents.is_some());
        assert_eq!(contents.unwrap().len(), 362958);
    }

    #[test]
    fn test_read_contents_missing_path() {
        assert!(read_contents("missing").is_none());
    }

    #[tokio::test]
    async fn test_byte_stream_from_string() {
        let expected_str = "hello, this is the expected string";

        let stream = byte_stream_from_string(expected_str);

        let data = collect_byte_stream(stream).await;
        assert_eq!(data, expected_str.as_bytes());
    }

    #[tokio::test]
    async fn test_byte_stream_from_empty_string() {
        let mut stream = byte_stream_from_string("");

        let bytes  = stream.next().await.unwrap();
        assert_eq!(bytes.len(), 0);
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_byte_stream_from_fs() {
        let stream = byte_stream_from_fs("../resources/jpg/PA280041.JPG").await.unwrap();

        let data = collect_byte_stream(stream).await;
        assert_eq!(data.len(), 362958);
    }

    #[tokio::test]
    async fn test_byte_stream_from_fs_missing_path() {
        assert!(byte_stream_from_fs("missing").await.is_err());
    }

    #[test]
    fn test_random_bytes() {
        let bytes = random_bytes(0);
        assert!(bytes.is_empty());

        let bytes = random_bytes(100);
        assert_eq!(bytes.len(), 100);
    }

    #[tokio::test]
    async fn test_random_byte_stream() {
        let (expected_bytes, stream) = random_byte_stream(100);

        let data = collect_byte_stream(stream).await;
        assert_eq!(data.len(), 100);
        assert_eq!(data, *expected_bytes);
    }

    #[tokio::test]
    async fn test_string_as_byte_stream() {
        let expected_str = "hello, this is the expected string";

        let stream = string_as_byte_stream(expected_str);

        let data = collect_byte_stream(stream).await;
        assert_eq!(data, expected_str.as_bytes());
    }
}