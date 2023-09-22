use std::borrow::Cow;
use async_trait::async_trait;
use bytes::Bytes;

use tokio_stream::Stream;

use crate::processing::ProcessContext;

/// Process is a trait that defines the interface for process data from a file or as raw bytes.
///
/// Process implementations are required to be thread safe.
///
#[async_trait]
pub trait Process: Send + Sync {
    /// Handle raw bytes.
    ///
    /// # Arguments
    ///
    /// * `content` - Async reader of the raw bytes to process.
    /// * `context` - The context for the processing operation.
    ///
    async fn process(&self, content: impl Stream<Item=Bytes> + Send + Sync + Unpin, context: ProcessContext) -> anyhow::Result<()>;
}
