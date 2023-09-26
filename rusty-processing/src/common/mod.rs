use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::pin::Pin;

use bytes::Bytes;
use futures::Stream;

pub(crate) use readable_stream::*;

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
pub type ByteStream = Pin<Box<dyn Stream<Item=Bytes> + Send + Sync>>;

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
pub fn write_to_file(contents: &[u8], path: &PathBuf) -> anyhow::Result<()> {
    fs::create_dir_all(path.parent().unwrap())?;
    File::create(path)?.write_all(contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_to_file() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?.into_path();
        let path = temp_dir.join("services.txt");
        let contents = b"Hello, world!";
        write_to_file(contents, &path)?;

        assert_eq!(fs::read(&path)?, contents);
        Ok(())
    }
}
