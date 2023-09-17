//!
//! Library for process files
//!
//! This library provides a framework for process files. It also provides a default processor that can be used
//! in applications.
//!
#![warn(missing_docs)]

/// Common functionality across the library
///
/// Contains things like common errors, traits, and types
///
pub mod common;

/// Contains the core logic and interface for process files.
///
/// Provides the all-purpose processor that can be used to process all implemented file types.
///
pub mod process;

pub(crate) mod message {
    pub mod rfc822;
}

pub(crate) mod application {
    pub mod mbox;
}

#[cfg(test)]
pub mod test_util;
