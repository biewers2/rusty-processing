use std::io::Read;

use crate::processing::ProcessContext;

/// Process is a trait that defines the interface for process data from a file or as raw bytes.
///
/// Process implementations are required to be thread safe.
///
pub trait Process: Send + Sync {
    /// Handle raw bytes.
    ///
    /// # Arguments
    ///
    /// * `content` - Async reader of the raw bytes to process.
    /// * `context` - The context for the processing operation.
    ///
    fn process(&self, content: Box<dyn Read + Send + Sync>, context: ProcessContext);
}
