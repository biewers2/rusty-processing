use std::future::Future;
use std::io::{Cursor, Read, Seek, Write};
use std::pin::Pin;

use bytes::Bytes;
use bytesize::MB;
use futures::{Stream, StreamExt};
use lazy_static::lazy_static;
use tempfile::NamedTempFile;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_stream::wrappers::ReceiverStream;

/// A representation of a stream of `bytes::Bytes`
///
/// Defines the type as a pointer to a stream that can be sent across threads and is sync and unpin.
///
pub type ByteStream = Pin<Box<dyn Stream<Item=Bytes> + Send + Sync>>;

type SyncReader = Box<dyn Read + Send + Unpin>;
type AsyncReader = Box<dyn AsyncRead + Send + Unpin>;

lazy_static! {
    static ref RUNTIME: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");
}

/// Global asynchronous runtime.
///
pub fn runtime() -> &'static tokio::runtime::Runtime {
    &RUNTIME
}

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

/// Creates a temporary file and returns its path.
///
pub fn temp_path() -> anyhow::Result<tempfile::TempPath> {
    Ok(NamedTempFile::new()?.into_temp_path())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::test_utils::random_bytes;

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

    // mod test_stream_reader {
    //     use crate::test_utils::{byte_stream_from_string, random_byte_stream};
    //
    //     use super::*;
    //
    //     #[test]
    //     fn test_read_empty_buffer() -> anyhow::Result<()> {
    //         let stream = random_byte_stream(10);
    //         let mut reader = StreamReader::new(stream);
    //
    //         let mut empty_buf: [u8; 0] = [];
    //         assert_eq!(0, reader.read(&mut empty_buf)?);
    //
    //         Ok(())
    //     }
    //
    //     #[test]
    //     fn test_read_smaller_buffer() -> anyhow::Result<()> {
    //         let text = "this is a stream of bytes from text, and it needs to be 100 characters long abcdefghijklmnopqrstuvwx";
    //         let text_as_bytes = text.as_bytes();
    //
    //         let stream = byte_stream_from_string(text);
    //         let mut reader = StreamReader::new(stream);
    //
    //         let mut buf = [0; 70];
    //         assert_eq!(70, reader.read(&mut buf)?);
    //         assert_eq!(text_as_bytes[..70], buf[..]);
    //         assert_eq!(30, reader.read(&mut buf)?);
    //         assert_ne!(text_as_bytes[70..], buf[..]);
    //         assert_eq!(0, reader.read(&mut buf)?);
    //
    //         Ok(())
    //     }
    //
    //     #[test]
    //     fn test_read_larger_buffer() -> anyhow::Result<()> {
    //         let text = "this is a stream of bytes from text, and it needs to be 100 characters long abcdefghijklmnopqrstuvwx";
    //         let text_as_bytes = text.as_bytes();
    //
    //         let stream = byte_stream_from_string(text);
    //         let mut reader = StreamReader::new(stream);
    //
    //         let mut buf = [0; 120];
    //         assert_eq!(100, reader.read(&mut buf)?);
    //         assert_eq!(text_as_bytes[..], buf[..100]);
    //         assert_eq!([0; 20], buf[100..]);
    //         assert_eq!(0, reader.read(&mut buf)?);
    //
    //         Ok(())
    //     }
    //
    //     #[test]
    //     fn test_read_no_data() -> anyhow::Result<()> {
    //         let stream = random_byte_stream(0);
    //         let mut reader = StreamReader::new(stream);
    //
    //         let mut buf = [0; 100];
    //         assert_eq!(0, reader.read(&mut buf)?);
    //         assert_eq!([0; 100], buf);
    //
    //         Ok(())
    //     }
    // }
}
