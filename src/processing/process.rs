use std::path;

/// Process is a trait that defines the interface for processing data from a file or as raw bytes.
///
/// Process implementations are required to be thread safe.
///
pub trait Process: Send + Sync {
    /// Handle a file.
    ///
    /// # Arguments
    ///
    /// * `source_file` - The path to the file to process.
    ///
    fn handle_file(&self, source_file: &path::PathBuf);

    /// Handle raw bytes.
    ///
    /// # Arguments
    ///
    /// * `raw` - The raw bytes to process.
    ///
    fn handle_raw(&self, raw: &[u8]);
}
