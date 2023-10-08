use std::io::Read;
use std::path;
use std::pin::Pin;
use bytes::Bytes;
use rand::Rng;
use tokio::io::AsyncReadExt;
use tokio_stream::Stream;

// Define the same type alias as found in `streaming/src/lib.rs`, as importing that crate here
// would create a circular dependency.
type ByteStream = Pin<Box<dyn Stream<Item=Bytes> + Send + Sync>>;

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
