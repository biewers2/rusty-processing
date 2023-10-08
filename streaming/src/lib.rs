use std::future::Future;
use std::io::{Cursor, Read, Seek, Write};
use std::pin::Pin;

use bytes::Bytes;
use bytesize::MB;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_stream::{Stream, StreamExt};
use tokio_stream::wrappers::ReceiverStream;

/// A representation of a stream of `bytes::Bytes`
///
/// Defines the type as a pointer to a stream that can be sent across threads and is sync and unpin.
///
pub type ByteStream = Pin<Box<dyn Stream<Item=Bytes> + Send + Sync>>;

pub type SyncReader = Box<dyn Read + Send + Unpin>;
pub type AsyncReader = Box<dyn AsyncRead + Send + Unpin>;

/// Given an object that's `Read`, create a byte stream to stream bytes from that object.
///
/// # Arguments
///
/// * `source` - The object to read from.
///
/// # Returns
///
/// * `Ok((ByteStream, Future))` - If the object was read successfully, `ByteStream` is the stream of bytes from the `Read` object,
///    and the `Future` represents the concurrent operation doing the actual reading from the object.
/// * `Err(_)` - If there was an error reading from the object.
///
pub fn read_to_stream(mut source: SyncReader) -> anyhow::Result<(ByteStream, impl Future<Output=anyhow::Result<()>>)> {
    let (sink, stream) = tokio::sync::mpsc::channel(100);

    let reading = async move {
        let mut buf = Box::new([0; MB as usize]);

        loop {
            let bytes_read = source.read(buf.as_mut())?;
            if bytes_read == 0 {
                break;
            }
            let bytes = Bytes::copy_from_slice(&buf[..bytes_read]);
            sink.send(bytes).await?;
        }

        anyhow::Ok(())
    };

    let stream = Box::pin(ReceiverStream::new(stream));
    Ok((stream, reading))
}

/// Given an object that's `AsyncRead`, create a byte stream to stream bytes from that object.
///
/// # Arguments
///
/// * `source` - The object to read from.
///
/// # Returns
///
/// * `Ok((ByteStream, Future))` - If the object was read successfully, `ByteStream` is the stream of bytes from the `AsyncRead` object,
///   and the `Future` represents the concurrent operation doing the actual reading from the object.
/// * `Err(_)` - If there was an error reading from the object.
///
pub fn async_read_to_stream(mut source: AsyncReader) -> anyhow::Result<(ByteStream, impl Future<Output=anyhow::Result<()>>)> {
    let (sink, stream) = tokio::sync::mpsc::channel(100);

    let reading = async move {
        let mut buf = Box::new([0; MB as usize]);

        loop {
            let bytes_read = source.read(buf.as_mut()).await?;
            if bytes_read == 0 {
                break;
            }
            let bytes = Bytes::copy_from_slice(&buf[..bytes_read]);
            sink.send(bytes).await?;
        }

        anyhow::Ok(())
    };

    let stream = Box::pin(ReceiverStream::new(stream));
    Ok((stream, reading))
}

/// Given a stream of bytes, create a `Read` object to read from that stream.
///
/// # Arguments
///
/// * `stream` - The stream of bytes to read from.
///
/// # Returns
///
/// * `Ok(SyncReader)` - If the stream was converted successfully, `SyncReader` is the `Read` object to read from the stream.
/// * `Err(_)` - If there was an error converting the stream.
///
pub async fn stream_to_read(mut stream: ByteStream) -> anyhow::Result<SyncReader> {
    const THRESHOLD: usize = MB as usize;

    let mut data = Vec::new();
    while let Some(bytes) = stream.next().await {
        let mut bytes = bytes.to_vec();
        data.append(&mut bytes);
        if data.len() >= THRESHOLD {
            return stream_remaining_to_file(stream, data).await;
        }
    }

    Ok(Box::new(Cursor::new(data)))
}

pub async fn stream_to_string(mut stream: ByteStream) -> String {
    let mut data = String::new();
    while let Some(bytes) = stream.next().await {
        let str = String::from_utf8_lossy(bytes.as_ref());
        data.push_str(&str);
    }
    data
}

/// Stream the remaining content of a byte stream to a file if there's too much data coming in to fill
/// into memory. `data_read` is the data that was already read from the stream by the caller.
///
async fn stream_remaining_to_file(mut stream: ByteStream, data_read: Vec<u8>) -> anyhow::Result<SyncReader> {
    let mut file = tempfile::tempfile()?;
    let mut read = Box::new(Cursor::new(data_read));
    std::io::copy(&mut read, &mut file)?;

    while let Some(bytes) = stream.next().await {
        file.write_all(&bytes)?;
    }

    file.flush()?;
    file.rewind()?;
    Ok(Box::new(file))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use tokio_stream::StreamExt;
    use test_utils::{random_bytes, string_as_byte_stream};

    use super::*;

    async fn collect_stream(mut stream: ByteStream) -> Box<Vec<u8>> {
        let mut bytes = Box::<Vec<u8>>::default();
        while let Some(b) = stream.next().await {
            bytes.append(&mut b.to_vec());
        }
        bytes
    }

    #[tokio::test]
    async fn test_empty_read_to_stream() -> anyhow::Result<()> {
        let expected_bytes = random_bytes(0);
        let reader = Box::new(Cursor::new(*expected_bytes.clone()));

        let (stream, reading) = read_to_stream(reader)?;
        let reading = tokio::spawn(reading);
        let bytes = collect_stream(stream).await;
        reading.await??;

        assert_eq!(expected_bytes, bytes);
        Ok(())
    }

    #[tokio::test]
    async fn test_read_to_stream() -> anyhow::Result<()> {
        let expected_bytes = random_bytes(100);
        let reader = Box::new(Cursor::new(*expected_bytes.clone()));

        let (stream, reading) = read_to_stream(reader)?;
        let reading = tokio::spawn(reading);
        let bytes = collect_stream(stream).await;
        reading.await??;

        assert_eq!(expected_bytes, bytes);
        Ok(())
    }

    #[tokio::test]
    async fn test_empty_async_read_to_stream() -> anyhow::Result<()> {
        let expected_bytes = random_bytes(0);
        let reader = Box::new(Cursor::new(*expected_bytes.clone()));

        let (stream, reading) = async_read_to_stream(reader)?;
        let reading = tokio::spawn(reading);
        let bytes = collect_stream(stream).await;
        reading.await??;

        assert_eq!(expected_bytes, bytes);
        Ok(())
    }

    #[tokio::test]
    async fn test_async_read_to_stream() -> anyhow::Result<()> {
        let expected_bytes = random_bytes(100);
        let reader = Box::new(Cursor::new(*expected_bytes.clone()));

        let (stream, reading) = async_read_to_stream(reader)?;
        let reading = tokio::spawn(reading);
        let bytes = collect_stream(stream).await;
        reading.await??;

        assert_eq!(expected_bytes, bytes);
        Ok(())
    }

    #[tokio::test]
    async fn test_stream_to_string() {
        let expected = "Hello, world!";
        let stream = string_as_byte_stream(expected);

        let string = stream_to_string(stream).await;

        assert_eq!(expected, string);
    }
}