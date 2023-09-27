use std::future::Future;
use std::io::Read;
use std::ops::Deref;
use std::pin::Pin;

use bytes::Bytes;
use futures::{Stream, StreamExt, TryFutureExt};
use futures::executor::block_on;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_stream::wrappers::ReceiverStream;

use crate::common::ByteStream;

type SyncReader = Box<dyn Read + Send + Unpin>;
type AsyncReader = Box<dyn AsyncRead + Send + Unpin>;

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
    read_to_stream_helper(move |buf| {
            source.read(buf.as_mut()).map_err(anyhow::Error::new)
    })
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
    read_to_stream_helper(move |buf| {
        block_on(source.read(buf).map_err(anyhow::Error::new))
    })
}

/// Helper function to create a byte stream where the reading is done through the provided `read` function.
///
fn read_to_stream_helper(mut read: impl FnMut(&mut [u8]) -> anyhow::Result<usize>) -> anyhow::Result<(ByteStream, impl Future<Output=anyhow::Result<()>>)> {
    let (sink, stream) = tokio::sync::mpsc::channel(100);

    let reading = async move {
        let mut buf = Box::new([0; 1024 * 1024]);

        loop {
            let bytes_read = read(buf.as_mut())?;
            if bytes_read == 0 {
                break;
            }
            println!("Transferring {} bytes", bytes_read);
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
pub fn stream_to_read(stream: ByteStream) -> SyncReader {
    Box::new(StreamReader::new(stream))
}

struct StreamReader<S: Stream<Item=Bytes> + Send + Sync> {
    inner: Pin<Box<S>>,
    buffer: Vec<u8>,
}

impl<S: Stream<Item=Bytes> + Send + Sync> StreamReader<S> {
    fn new(inner: S) -> Self {
        Self {
            inner: Box::pin(inner),
            buffer: Vec::new(),
        }
    }
}

impl<S: Stream<Item=Bytes> + Send + Sync> Deref for StreamReader<S> {
    type Target = Pin<Box<S>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<S: Stream<Item=Bytes> + Send + Sync> Read for StreamReader<S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let mut bytes_read = 0;
        while buf.len() - bytes_read > 0 {
            if self.buffer.is_empty() {
                // Refill buffer if empty
                let bytes = match futures::executor::block_on(self.inner.next()) {
                    Some(bytes) => bytes,
                    None => break,
                };
                self.buffer = bytes.to_vec();
            } else {
                // Otherwise, write out buffer as much as possible
                let bytes_to_copy = std::cmp::min(buf.len() - bytes_read, self.buffer.len());
                let ending_index = bytes_read + bytes_to_copy;
                buf[bytes_read..ending_index].copy_from_slice(&self.buffer[..bytes_to_copy]);
                self.buffer = self.buffer[bytes_to_copy..].to_vec();
                bytes_read += bytes_to_copy;
            }
        }

        Ok(bytes_read)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use crate::common::StreamReader;
    use crate::test_util::{byte_stream_from_string, random_byte_stream};

    #[test]
    fn test_read_empty_buffer() -> anyhow::Result<()> {
        let stream = random_byte_stream(10);
        let mut reader = StreamReader::new(stream);

        let mut empty_buf: [u8; 0] = [];
        assert_eq!(0, reader.read(&mut empty_buf)?);

        Ok(())
    }

    #[test]
    fn test_read_smaller_buffer() -> anyhow::Result<()> {
        let text = "this is a stream of bytes from text, and it needs to be 100 characters long abcdefghijklmnopqrstuvwx";
        let text_as_bytes = text.as_bytes();

        let stream = byte_stream_from_string(text);
        let mut reader = StreamReader::new(stream);

        let mut buf = [0; 70];
        assert_eq!(70, reader.read(&mut buf)?);
        assert_eq!(text_as_bytes[..70], buf[..]);
        assert_eq!(30, reader.read(&mut buf)?);
        assert_ne!(text_as_bytes[70..], buf[..]);
        assert_eq!(0, reader.read(&mut buf)?);

        Ok(())
    }

    #[test]
    fn test_read_larger_buffer() -> anyhow::Result<()> {
        let text = "this is a stream of bytes from text, and it needs to be 100 characters long abcdefghijklmnopqrstuvwx";
        let text_as_bytes = text.as_bytes();

        let stream = byte_stream_from_string(text);
        let mut reader = StreamReader::new(stream);

        let mut buf = [0; 120];
        assert_eq!(100, reader.read(&mut buf)?);
        assert_eq!(text_as_bytes[..], buf[..100]);
        assert_eq!([0; 20], buf[100..]);
        assert_eq!(0, reader.read(&mut buf)?);

        Ok(())
    }

    #[test]
    fn test_read_no_data() -> anyhow::Result<()> {
        let stream = random_byte_stream(0);
        let mut reader = StreamReader::new(stream);

        let mut buf = [0; 100];
        assert_eq!(0, reader.read(&mut buf)?);
        assert_eq!([0; 100], buf);

        Ok(())
    }
}