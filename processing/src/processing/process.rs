use async_trait::async_trait;
use crate::io::ByteStream;

use crate::processing::ProcessContext;

/// Process is a trait that defines the interface for process data from a file or as raw bytes.
///
/// Process implementations are required to be thread safe.
///
#[async_trait]
pub trait Process: Send + Sync {
    /// Process a stream of bytes.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of the processing operation.
    /// * `content` - Async reader of the raw bytes to process.
    ///
    async fn process(&self, ctx: ProcessContext, stream: ByteStream) -> anyhow::Result<()>;
}
