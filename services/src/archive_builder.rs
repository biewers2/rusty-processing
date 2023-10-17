use std::fs::File;
use std::io::{Read, Write};
use std::path;
use std::path::Path;

use bytesize::MB;

/// A builder for creating an archive.
///
/// This builder eagerly writes the contents to an archive.
///
pub struct ArchiveBuilder {
    zipper: zip::ZipWriter<File>,
}

impl ArchiveBuilder {
    /// Create a new archive builder.
    ///
    pub fn new(file: File) -> anyhow::Result<Self> {
        let zipper = zip::ZipWriter::new(file);

        Ok(Self { zipper })
    }

    /// Add a file to the archive.
    ///
    /// # Arguments
    ///
    /// * `input_path` - The path to the file to add to the archive.
    /// * `zip_path` - The path to the file in the archive.
    ///
    pub fn push(&mut self, input_path: impl AsRef<Path>, zip_path: impl AsRef<Path>) -> anyhow::Result<()> {
        let zip_path_str = zip_path.as_ref().to_string_lossy();
        self.zipper.start_file(zip_path_str, Default::default())?;

        let path = input_path.as_ref();
        self.write_file(path)?;

        Ok(())
    }

    /// Build the archive.
    ///
    pub fn build(&mut self) -> anyhow::Result<std::fs::File> {
        Ok(self.zipper.finish()?)
    }

    fn write_file(&mut self, path: &path::Path) -> anyhow::Result<()> {
        let mut file = std::fs::File::open(path)?;

        let mut buf = Box::new([0; MB as usize]);
        loop {
            let bytes_read = file.read(buf.as_mut())?;
            if bytes_read == 0 {
                break;
            }
            self.zipper.write_all(&buf[..bytes_read])?;
        }
        Ok(())
    }
}