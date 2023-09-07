/// Defines a trait to provide a common interface for identifying duplicate files.
///
pub mod identify;

/// Identifier provides specific identification implementations based on a file's MIME type.
///
pub mod identifier;

/// Implementation of [`identify_dupe::IdentifyDupe`] that uses the MD5 hash of the file contents.
///
pub(crate) mod md5_dupe_identifier;

/// Implementation of [`identify_dupe::IdentifyDupe`] that hashes the message ID of the file.
///
/// Uses the [`md5_dupe_identifier::Md5DupeIdentifier`].
///
pub(crate) mod message_dupe_identifier;
