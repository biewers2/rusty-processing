use std::path;

use serde::{Deserialize, Serialize};
use tempfile::{tempdir, NamedTempFile};
use temporal_sdk::ActContext;

#[derive(Deserialize, Debug)]
pub struct CreateWorkspaceInput {}

#[derive(Serialize, Debug)]
pub struct CreateWorkspaceOutput {
    pub source_path: path::PathBuf,
    pub output_dir: path::PathBuf,
}

pub async fn create_workspace(
    _ctx: ActContext,
    _: CreateWorkspaceInput,
) -> anyhow::Result<CreateWorkspaceOutput> {
    let output_dir = tempdir()?;
    let source_path = NamedTempFile::new_in(&output_dir)?;
    Ok(CreateWorkspaceOutput {
        source_path: source_path.into_temp_path().to_path_buf(),
        output_dir: output_dir.into_path(),
    })
}
