//!
//! Library for processing files
//!
//! This library provides a framework for processing files. It also provides a default processor that can be used
//! in applications.
//!
#![warn(missing_docs)]

/// Common functionality across the library
///
/// Contains things like common errors, traits, and types
///
pub mod common;

/// Contains the core logic and interface for processing files.
///
/// Provides the all-purpose processor that can be used to process all implemented file types.
///
pub mod processing;

pub(crate) mod application {
    #[cfg(feature = "mail")]
    pub mod mbox;
}

#[cfg(feature = "mail")]
pub(crate) mod message {
    pub mod rfc822;
}

#[cfg(test)]
pub mod test_util;
