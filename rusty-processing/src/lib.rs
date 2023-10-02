//!
//! Library for processing files
//!
//! This library provides a framework for processing files. It also provides a default processor that can be used
//! in applications.
//!
#![warn(missing_docs)]

/// Contains the core logic and interface for processing files.
///
/// Provides the all-purpose processor that can be used to process all implemented file types.
///
pub mod processing;

/// Contains I/O related functionality.
///
pub mod io;

pub(crate) mod services;

pub(crate) mod application {
    #[cfg(feature = "mail")]
    pub mod mbox;
}

#[cfg(feature = "mail")]
pub(crate) mod message {
    pub mod rfc822;
}

pub(crate) mod workspace {
    use std::io::Write;
    use tempfile::TempPath;
    use crate::io::temp_path;
    use crate::processing::ProcessType;

    /// A workspace quickly creates a set of files that can be used when operating on a file.
    ///
    /// `original_file` is the path to the file w/ the provided contents
    /// `text_file` is the path to where the extracted text should be written (file won't exist yet)
    /// `metadata_file` is the path to where the metadata JSON should be written (file won't exist yet)
    /// `pdf_file` is the path to where the rendered PDF should be written (file won't exist yet)
    ///
    #[derive(Debug)]
    pub struct Workspace {
        pub original_path: TempPath,
        pub text_path: Option<TempPath>,
        pub metadata_path: Option<TempPath>,
        pub pdf_path: Option<TempPath>,
    }

    impl Workspace {
        pub fn new(content: &[u8], types: &[ProcessType]) -> anyhow::Result<Workspace> {
            let original_path = temp_path()?;
            let mut original_file = std::fs::File::create(&original_path)?;
            original_file.write_all(content)?;

            let text_path = types.contains(&ProcessType::Text).then(temp_path).transpose()?;
            let metadata_path = types.contains(&ProcessType::Metadata).then(temp_path).transpose()?;
            let pdf_path = types.contains(&ProcessType::Pdf).then(temp_path).transpose()?;

            Ok(Workspace {
                original_path,
                text_path,
                metadata_path,
                pdf_path,
            })
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn test_workspace_no_types() -> anyhow::Result<()> {
            let workspace = Workspace::new(b"hello, world!", &[])?;

            assert!(workspace.text_path.is_none());
            assert!(workspace.metadata_path.is_none());
            assert!(workspace.pdf_path.is_none());
            Ok(())
        }

        #[test]
        fn test_workspace_all_types() -> anyhow::Result<()> {
            let types = vec![ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf];
            let workspace = Workspace::new(b"hello, world!", &types)?;

            assert!(workspace.text_path.is_some());
            assert!(workspace.metadata_path.is_some());
            assert!(workspace.pdf_path.is_some());
            Ok(())
        }
    }

}

#[cfg(test)]
pub mod test_utils {
    use std::io::Read;
    use std::path;
    use bytes::Bytes;
    use rand::Rng;
    use tokio::io::AsyncReadExt;
    use crate::io::ByteStream;

    pub fn read_contents(path: &str) -> anyhow::Result<Vec<u8>> {
        let mut content = vec![];
        std::fs::File::open(path::PathBuf::from(path))?.read_to_end(&mut content)?;
        Ok(content)
    }

    pub fn byte_stream_from_string(value: impl Into<String>) -> ByteStream {
        let bytes = Bytes::from(value.into());
        Box::pin(async_stream::stream! { yield bytes })
    }

    pub async fn byte_stream_from_fs(path: path::PathBuf) -> anyhow::Result<ByteStream> {
        let file = tokio::fs::File::open(path).await.unwrap();
        let mut reader = tokio::io::BufReader::new(file);

        let mut buf = vec![];
        reader.read_to_end(&mut buf).await?;
        let bytes = Bytes::from(buf);
        let stream = Box::pin(async_stream::stream! { yield bytes });

        Ok(stream)
    }

    pub fn random_bytes(len: usize) -> Box<Vec<u8>> {
        let mut rng = rand::thread_rng();
        Box::new((0..len).map(|_| rng.gen()).collect::<Vec<u8>>())
    }

    pub fn random_byte_stream(len: usize) -> ByteStream {
        let bytes = Bytes::from(*random_bytes(len));
        Box::pin(async_stream::stream! { yield bytes })
    }
}
