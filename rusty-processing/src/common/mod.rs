use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use bytes::Bytes;
use futures::Stream;
use mail_parser::ContentType;

pub(crate) use readable_stream::*;

/// Service for converting HTML to PDF.
///
pub(crate) mod wkhtmltopdf;

/// Service for creating a "workspace" for processing files.
///
/// A workspace is a location inside a specified directory that is used to store the file output from processing.
///
pub(crate) mod workspace;

mod readable_stream;

/// A representation of a stream of `bytes::Bytes`
/// 
/// Defines the type as a pointer to a stream that can be sent across threads and is sync and unpin.
/// 
pub type ByteStream = Box<dyn Stream<Item=Bytes> + Send + Sync + Unpin>;

/// Write the `contents` to a newly created file at `path` recursively creating any parent directories.
/// 
/// # Arguments
/// 
/// * `path` - The path to the file to write to.
/// * `contents` - The contents to write to the file.
/// 
/// # Errors
/// 
/// Returns an error if the file cannot be created or written to.
/// 
pub fn write_file(path: &PathBuf, contents: &[u8]) -> anyhow::Result<()> {
    fs::create_dir_all(path.parent().unwrap())?;
    File::create(path)?.write_all(contents)?;
    Ok(())
}

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

    use super::*;

    #[test]
    fn test_write_file() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?.into_path();
        let path = temp_dir.join("services.txt");
        let contents = b"Hello, world!";
        write_file(&path, contents)?;

        assert_eq!(fs::read(&path)?, contents);
        Ok(())
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
