use std::io::Read;
use std::ops::Deref;
use std::pin::Pin;
use bytes::Bytes;
use futures::{Stream, StreamExt};

pub struct StreamReader<S: Stream<Item=Bytes> + Send + Sync> {
    inner: Pin<Box<S>>,
    buffer: Vec<u8>,
}

impl<S: Stream<Item=Bytes> + Send + Sync> StreamReader<S> {
    pub fn new(inner: S) -> Self {
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