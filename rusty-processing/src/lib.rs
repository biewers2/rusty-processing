#![warn(missing_docs)]
//!
//! Library for processing files
//!
//! This library provides a framework for processing files. It also provides a default processor that can be used
//! in applications.
//!

/// Common functionality across the library
///
/// Contains things like common errors, traits, and types
///
pub mod common;

/// Composed of identifiers used to calculate the deduplication hash of a file.
///
pub mod dupe_id;

/// Contains the core logic and interface for processing files.
///
/// Provides the all-purpose processor that can be used to process all implemented file types.
///
pub mod processing;

pub(crate) mod message {
    pub mod rfc822;
}
pub(crate) mod application {
    pub mod mbox;
}
