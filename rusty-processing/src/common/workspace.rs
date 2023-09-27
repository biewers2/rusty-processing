use std::io::Write;

use tempfile::{tempdir, TempPath};

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
        let working_dir = tempdir()?.into_path();

        let original_path = working_dir.join("original");
        let mut original_file = std::fs::File::create(&original_path)?;
        original_file.write_all(content)?;

        let text_path = types.contains(&ProcessType::Text).then(|| working_dir.join("extracted.txt"));
        let metadata_path = types.contains(&ProcessType::Metadata).then(|| working_dir.join("metadata.json"));
        let pdf_file = types.contains(&ProcessType::Pdf).then(|| working_dir.join("rendered.pdf"));

        Ok(Workspace {
            original_path: TempPath::from_path(original_path),
            text_path: text_path.map(TempPath::from_path),
            metadata_path: metadata_path.map(TempPath::from_path),
            pdf_path: pdf_file.map(TempPath::from_path),
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
