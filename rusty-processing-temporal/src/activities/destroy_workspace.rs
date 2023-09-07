use serde::{Deserialize, Serialize};
use std::{fs, path};

use temporal_sdk::ActContext;

#[derive(Deserialize, Debug)]
pub struct DestroyWorkspaceInput {
    pub source_path: path::PathBuf,
    pub output_dir: path::PathBuf,
}

#[derive(Serialize, Debug)]
pub struct DestroyWorkspaceOutput;

pub async fn destroy_workspace(
    _ctx: ActContext,
    input: DestroyWorkspaceInput,
) -> anyhow::Result<DestroyWorkspaceOutput> {
    fs::remove_file(input.source_path)?;
    fs::remove_dir_all(input.output_dir)?;
    Ok(DestroyWorkspaceOutput {})
}
