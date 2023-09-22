use std::io::Read;
use std::ops::Deref;
use bytes::Bytes;
use futures::{Stream, StreamExt};

pub struct StreamReader<S: Stream<Item=Bytes> + Send + Sync + Unpin> {
    inner: Box<S>,
    buffer: Vec<u8>,
}

impl<S: Stream<Item=Bytes> + Send + Sync + Unpin> StreamReader<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner: Box::new(inner),
            buffer: Vec::new(),
        }
    }
}

impl<S: Stream<Item=Bytes> + Send + Sync + Unpin> Deref for StreamReader<S> {
    type Target = Box<S>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<S: Stream<Item=Bytes> + Send + Sync + Unpin> Read for StreamReader<S> {
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
                    None => return Ok(0),
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