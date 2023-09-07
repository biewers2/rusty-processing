use std::fs::File;
use std::io::Write;
use std::path;
use std::path::PathBuf;

use crate::common::mime_extension_map::map_to_file_ext;
use crate::common::output_type::ProcessType;
use crate::common::util;
use crate::dupe_id::identify_dupe::identifier;
use crate::processing::context::Context;

/// A workspace defines a directory tree schematic.
///
/// Given a context containing an output dir, it takes the content of a file and creates a set of files and file paths that
/// can be used when operating on that file (i.e. extracting text, metadata, etc.).
///
/// The schematic is defined as such:
///   ${context.output_dir}/
///     <dupe_id_of_original>/
///       original.<ext>
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
    pub fn new(context: &Context, content: &[u8]) -> anyhow::Result<Workspace> {
        let dupe_id = identifier(&context.mimetype).identify(content);
        let dir = context.output_dir.join(&dupe_id);

        let original_path = dir.join(format!("original.{}", map_to_file_ext(&context.mimetype)));
        util::write_file(&original_path, content)?;

        let text_path = context
            .should_process_type(&ProcessType::Text)
            .then(|| dir.join("extracted.txt"))
            .and_then(|path| (!path.exists()).then(|| path));
        let metadata_path = context
            .should_process_type(&ProcessType::Metadata)
            .then(|| dir.join("metadata.json"))
            .and_then(|path| (!path.exists()).then(|| path));
        let pdf_path = context
            .should_process_type(&ProcessType::Pdf)
            .then(|| dir.join("rendered.pdf"))
            .and_then(|path| (!path.exists()).then(|| path));

        Ok(Workspace {
            dupe_id,
            entry_dir: dir,
            original_path,
            text_path,
            metadata_path,
            pdf_path,
        })
    }

    /// Creates a writer for the extracted text to be written to.
    /// 
    pub fn text_writer(&self) -> anyhow::Result<Option<Box<dyn Write>>> {
        self.writer(&self.text_path)
    }
    
    /// Creates a writer for the metadata JSON to be written to.
    /// 
    pub fn metadata_writer(&self) -> anyhow::Result<Option<Box<dyn Write>>> {
        self.writer(&self.metadata_path)
    }
    
    /// Creates a writer for the rendered PDF to be written to.
    /// 
    pub fn pdf_writer(&self) -> anyhow::Result<Option<Box<dyn Write>>> {
        self.writer(&self.pdf_path)
    }
    
    fn writer(&self, path: &Option<PathBuf>) -> anyhow::Result<Option<Box<dyn Write>>> {
        Ok(path.as_ref()
            .map(|path| File::create(&path)).transpose()?
            .map(|file| Box::new(file) as Box<dyn Write>))
    }
}

#[cfg(test)]
mod test {
    use std::sync::Mutex;

    use super::*;

    #[test]
    fn test_workspace_no_types() -> anyhow::Result<()> {
        let output_dir = tempfile::tempdir()?.into_path();
        let dupe_id = "3adbbad1791fbae3ec908894c4963870";
        let dir = output_dir.join(dupe_id);
        let ctx = Context {
            output_dir,
            mimetype: "text/plain".to_string(),
            types: vec![],
            result_tx: Mutex::new(None),
        };

        let workspace = Workspace::new(&ctx, b"hello, world!")?;

        assert_eq!(workspace.dupe_id, dupe_id);
        assert_eq!(workspace.entry_dir, dir);
        assert_eq!(workspace.original_path, dir.join("original.txt"));
        assert_eq!(workspace.text_path, None);
        assert_eq!(workspace.metadata_path, None);
        assert_eq!(workspace.pdf_path, None);
        Ok(())
    }

    #[test]
    fn test_workspace_all_types() -> anyhow::Result<()> {
        let output_dir = tempfile::tempdir()?.into_path();
        let dupe_id = "3adbbad1791fbae3ec908894c4963870";
        let dir = output_dir.join(dupe_id);
        let ctx = Context {
            output_dir,
            mimetype: "text/plain".to_string(),
            types: vec![ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf],
            result_tx: Mutex::new(None),
        };

        let workspace = Workspace::new(&ctx, b"hello, world!")?;

        assert_eq!(workspace.dupe_id, dupe_id);
        assert_eq!(workspace.entry_dir, dir);
        assert_eq!(workspace.original_path, dir.join("original.txt"));
        assert_eq!(workspace.text_path, Some(dir.join("extracted.txt")));
        assert_eq!(workspace.metadata_path, Some(dir.join("metadata.json")));
        assert_eq!(workspace.pdf_path, Some(dir.join("rendered.pdf")));
        Ok(())
    }
}
