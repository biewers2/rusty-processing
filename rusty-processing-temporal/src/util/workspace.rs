use std::path;

use tempfile::{NamedTempFile, tempdir};

pub struct Workspace {
    pub source_path: path::PathBuf,
    pub output_dir: path::PathBuf,
}

impl Workspace {
    pub async fn new() -> anyhow::Result<Self> {
        let output_dir = tempdir()?;
        let source_path = NamedTempFile::new_in(&output_dir)?;
        Ok(Self {
            source_path: source_path.into_temp_path().to_path_buf(),
            output_dir: output_dir.into_path(),
        })
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.source_path);
        let _ = std::fs::remove_dir_all(&self.output_dir);
    }
}
