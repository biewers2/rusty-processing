//!
//! # Test Utilities
//!
#![warn(missing_docs)]

use std::io::Read;
use std::path;

use tempfile::{NamedTempFile, TempPath};

/// Reads the contents of a file into a `Vec<u8>`.
///
/// # Arguments
///
/// * `path` - The path to the file to read.
///
/// # Returns
///
/// Some contents of the file as a `Vec<u8>`, or None if the file could not be read.
///
pub fn read_contents(path: &str) -> Option<Vec<u8>> {
    let mut content = vec![];
    std::fs::File::open(path::PathBuf::from(path))
        .and_then(|mut file| file.read_to_end(&mut content))
        .map(|_| content)
        .ok()
}

/// Creates a temporary file and returns its path.
///
#[inline]
pub fn temp_path() -> std::io::Result<TempPath> {
    Ok(NamedTempFile::new()?.into_temp_path())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_contents() {
        let contents = read_contents("../resources/jpg/PA280041.JPG");
        assert!(contents.is_some());
        assert_eq!(contents.unwrap().len(), 362958);
    }

    #[test]
    fn test_read_contents_missing_path() {
        assert!(read_contents("missing").is_none());
    }
}