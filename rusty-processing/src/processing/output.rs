use serde::{Deserialize, Serialize};
use std::path;

/// Output is the result of processing a file.
///
/// It can be either a new file or an embedded file.
///
#[derive(Serialize, Deserialize, Debug)]
pub enum Output {
    /// A newly created file as a result of processing the original file.
    ///
    Processed(OutputInfo),

    /// A file discovered during the processing of the original file.
    ///
    Embedded(OutputInfo),
}

/// OutputInfo contains information about the output file.
///
/// It contains the path, mimetype and dupe_id.
///
#[derive(Serialize, Deserialize, Debug)]
pub struct OutputInfo {
    /// Path to the output file.
    ///
    pub path: path::PathBuf,

    /// Mimetype of the output file.
    ///
    pub mimetype: String,

    /// Dupe ID of the output file.
    ///
    pub dupe_id: String,
}
