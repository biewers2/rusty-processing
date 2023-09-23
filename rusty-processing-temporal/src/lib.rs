//! Code used by the Temporal worker.
//!
#![warn(missing_docs)]

/// Temporal activity definitions.
///
pub mod activities {
    /// Activity for processing a Rusty file.
    ///
    pub mod process_rusty_file;
}

/// I/O-related functionality.
///
pub(crate) mod io {
    /// Downloading a file from S3.
    ///
    pub mod download;

    /// Uploading a file to S3.
    ///
    pub mod upload;

    /// Uploader for uploading large files to S3 using multipart uploads.
    ///
    pub mod multipart_uploader;
}

/// Utility functionality.
///
pub(crate) mod util;

/// Services used by the Temporal worker.
///
pub(crate) mod services;
