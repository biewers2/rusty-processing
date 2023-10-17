pub use create_workspace::*;
pub use remove_workspace::*;
pub use process_rusty_file::*;
pub use download::*;
pub use upload::*;
pub use zip::*;

/// Activity for creating a workspace.
///
/// A workspace is a directory on the local filesystem that other activities can do
/// file processing I/O on.
///
mod create_workspace;

/// Activity for removing workspace files.
///
mod remove_workspace;

/// Activity for processing a Rusty file.
///
mod process_rusty_file;

/// Activity for downloading a file from S3.
/// 
mod download;

/// Activity for uploading a file to S3.
/// 
mod upload;

/// Activity for zipping up files in a directory.
///
mod zip;