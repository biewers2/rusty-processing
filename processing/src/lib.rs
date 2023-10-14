//!
//! Library for processing files
//!
//! This library provides a framework for processing files. It also provides a default processor that can be used
//! in applications.
//!
#![warn(missing_docs)]

use log::warn;
use mail_parser::ContentType;
use tempfile::{NamedTempFile, TempPath};

use crate::processing::ProcessType;

/// Contains the core logic and interface for processing files.
///
/// Provides the all-purpose processor that can be used to process all implemented file types.
///
pub mod processing;

pub(crate) mod text;
pub(crate) mod metadata;
pub(crate) mod pdf;
pub(crate) mod embedded;

/// Get the MIME type from a `mail_parser::ContentType`.
///
/// # Arguments
///
/// * `content_type` - The `mail_parser::ContentType` to get the MIME type from.
///
/// # Returns
///
/// The MIME type formatted as a `String`.
///
pub fn mimetype(content_type: &ContentType) -> String {
    match (content_type.ctype(), content_type.subtype()) {
        (ctype, Some(subtype)) => format!("{}/{}", ctype, subtype),
        (ctype, None) => ctype.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use mail_parser::ContentType;

    use super::*;

    #[test]
    fn test_build_paths_from_types_no_types() {
        let paths = build_paths_from_types(&[]).unwrap();

        assert!(paths.text.is_none());
        assert!(paths.metadata.is_none());
        assert!(paths.pdf.is_none());
    }

    #[test]
    fn test_build_paths_from_types_all_types() {
        let types = vec![ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf];
        let paths = build_paths_from_types(&types).unwrap();

        assert!(paths.text.is_some());
        assert!(paths.metadata.is_some());
        assert!(paths.pdf.is_some());
    }

    #[test]
    fn test_mimetype_with_subtype() {
        let content_type = ContentType {
            c_type: Cow::from("text"),
            c_subtype: Some(Cow::from("plain")),
            attributes: None,
        };

        assert_eq!(mimetype(&content_type), "text/plain");
    }

    #[test]
    fn test_mimetype_without_subtype() {
        let content_type = ContentType {
            c_type: Cow::from("text"),
            c_subtype: None,
            attributes: None,
        };

        assert_eq!(mimetype(&content_type), "text");
    }
}
