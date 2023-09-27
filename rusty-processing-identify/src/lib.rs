/// Defines a trait to provide a common interface for identifying duplicate files.
///
mod identify;

/// Identifier provides specific identification implementations based on a file's MIME type.
///
mod identifier;

/// Implementation of [`identify_dupe::IdentifyDupe`] that uses the MD5 hash of the file contents.
///
mod md5_dedupe_identifier;

/// Implementation of [`identify_dupe::IdentifyDupe`] that hashes the message ID of the file.
///
/// Uses the [`md5_dedupe_identifier::Md5DupeIdentifier`].
///
mod message_dedupe_identifier;

pub use identify::*;
pub use identifier::*;
pub(crate) use md5_dedupe_identifier::*;
pub(crate) use message_dedupe_identifier::*;
