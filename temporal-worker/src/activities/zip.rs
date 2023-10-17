use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};

use log::info;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use temporal_sdk::ActContext;

use services::ArchiveBuilder;

/// Input to the `zip` activity.
/// 
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZipInput {
    /// The S3 URI to download the file from.
    /// 
    pub directory: PathBuf,
}

/// Output from the `zip` activity.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZipOutput {
    /// The local path to where the zip file should be written to.
    ///
    pub path: PathBuf,
}

/// Activity for zipping up files in a directory.
///
pub async fn zip(_ctx: ActContext, input: ZipInput) -> anyhow::Result<ZipOutput> {
    if !input.directory.starts_with("/tmp") {
        return Err(anyhow::anyhow!("directory must be in /tmp"));
    }

    let path = NamedTempFile::new()?.into_temp_path().to_path_buf();
    let file = fs::File::create(&path)?;
    let mut builder = ArchiveBuilder::new(file)?;
    walk(&input.directory, &mut |entry| {
        let path = entry.path();
        let zip_path = path.strip_prefix(&input.directory)?;
        info!("adding {:?} into {:?}", path, zip_path);
        builder.push(&path, zip_path)
    })?;
    builder.build()?;
    Ok(ZipOutput { path })
}

fn walk(dir: impl AsRef<Path>, handle_file: &mut dyn FnMut(&DirEntry) -> anyhow::Result<()>) -> anyhow::Result<()> {
    let dir = dir.as_ref();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(path, handle_file)?;
            } else {
                handle_file(&entry)?;
            }
        }
    }
    Ok(())
}