//!
//! Identify is a library for identifying files based on their content.
//!
//! "Identification" includes identifying the following:
//! * De-duplication checksum
//! * MIME type
//!
#![warn(missing_docs)]

/// De-duplication functionality.
///
pub mod deduplication;

/// MIME type identification functionality.
///
pub mod mimetype;
