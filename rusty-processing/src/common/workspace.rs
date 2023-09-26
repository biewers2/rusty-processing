use std::path;

use rusty_processing_identify::identifier::identifier;
use crate::common::write_to_file;

use crate::processing::ProcessType;

/// A workspace defines a directory tree schematic.
///
/// Given a context containing an output dir, it takes the content of a file and creates a set of files and file paths that
/// can be used when operating on that file (i.e. extracting text, metadata, etc.).
///
/// The schematic is defined as such:
///   ${context.output_dir}/
///     <dupe_id_of_original>/
///       original
///       extracted.txt
///       metadata.json
///       rendered.pdf
///
/// `dupe_id` is the deduplication value of the original contents
/// `entry_dir` is the directory w/ `dupe_id` as the base (this directory is scoped to the original file)
/// `original_path` is the path to the file w/ the provided contents
/// `text_path` is the path to where the extracted text should be written (file won't exist yet)
/// `metadata_path` is the path to where the metadata JSON should be written (file won't exist yet)
/// `pdf_path` is the path to where the rendered PDF should be written (file won't exist yet)
///
#[derive(Debug)]
pub struct Workspace {
    pub dupe_id: String,
    pub entry_dir: path::PathBuf,
    pub original_path: path::PathBuf,
    pub text_path: Option<path::PathBuf>,
    pub metadata_path: Option<path::PathBuf>,
    pub pdf_path: Option<path::PathBuf>,
}

impl Workspace {
    /// Create a new workspace given a context containing the output directory, and the content of the original file to work on.
    ///
    pub fn new(
        content: &[u8],
        mimetype: impl AsRef<str>,
        types: &[ProcessType]
    ) -> anyhow::Result<Workspace> {
        let dupe_id = identifier(&mimetype).identify(content);

        let output_dir = tempfile::tempdir()?.into_path();
        let entry_dir = output_dir.join(&dupe_id);

        let original_path = entry_dir.join("original");
        write_to_file(content, &original_path)?;

        let text_path = types.contains(&ProcessType::Text)
            .then(|| entry_dir.join("extracted.txt"))
            .and_then(|path| (!path.exists()).then_some(path));
        let metadata_path = types.contains(&ProcessType::Metadata)
            .then(|| entry_dir.join("metadata.json"))
            .and_then(|path| (!path.exists()).then_some(path));
        let pdf_path = types.contains(&ProcessType::Pdf)
            .then(|| entry_dir.join("rendered.pdf"))
            .and_then(|path| (!path.exists()).then_some(path));

        Ok(Workspace {
            dupe_id,
            entry_dir,
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
        let mimetype = "text/plain".to_string();
        let dupe_id = "3adbbad1791fbae3ec908894c4963870";

        let workspace = Workspace::new(
            b"hello, world!",
            mimetype,
            &[],
        )?;

        let base = path::PathBuf::from(dupe_id);
        assert_eq!(workspace.dupe_id, dupe_id);
        assert!(workspace.original_path.ends_with(base.join("original")));
        assert_eq!(workspace.text_path, None);
        assert_eq!(workspace.metadata_path, None);
        assert_eq!(workspace.pdf_path, None);

        Ok(())
    }

    #[test]
    fn test_workspace_all_types() -> anyhow::Result<()> {
        let mimetype = "text/plain";
        let types = vec![ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf];
        let dupe_id = "3adbbad1791fbae3ec908894c4963870";

        let workspace = Workspace::new(
            b"hello, world!",
            mimetype,
            &types,
        )?;

        let base = path::PathBuf::from(dupe_id);
        assert_eq!(workspace.dupe_id, dupe_id);
        assert!(workspace.original_path.ends_with(base.join("original")));
        assert!(workspace.text_path.unwrap().ends_with(base.join("extracted.txt")));
        assert!(workspace.metadata_path.unwrap().ends_with(base.join("metadata.json")));
        assert!(workspace.pdf_path.unwrap().ends_with(base.join("rendered.pdf")));
        Ok(())
    }
}
