use std::fmt::Debug;
use std::path;

use serde::{Deserialize, Serialize};

/// OutputInfo contains information about the output file.
///
/// It contains the path, mimetype and dupe_id.
///
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct ProcessOutput {
    /// Path to the output file.
    ///
    pub path: path::PathBuf,

    /// Type of this output.
    ///
    pub output_type: ProcessOutputType,

    /// Mimetype of the output file.
    ///
    pub mimetype: String,

    /// Dupe ID of the output file.
    ///
    pub dupe_id: String,
}

/// Output is the result of processing a file.
///
/// It can be either a new file or an embedded file.
///
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum ProcessOutputType {
    /// A newly created file as a result of processing the original file.
    ///
    Processed,

    /// A file discovered during the processing of the original file.
    ///
    Embedded,
}
