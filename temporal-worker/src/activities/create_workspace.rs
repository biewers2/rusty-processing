use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use temporal_sdk::ActContext;
use crate::hostname;

/// Placeholder for the input to the `create_workspace` activity.
///
/// Required to allow this activity to be callable from external workflows.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceInput {}

/// Output from the `create_workspace` activity.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceOutput {
    /// The local path to where the initial file to process.
    ///
    pub root_path: PathBuf,

    /// The local path to the directory where output files should be written to.
    ///
    pub directory: PathBuf,

    /// The name of the task queue to use for operating on files in this workspace.
    ///
    /// This is used to force workflows and activities to run on the same machine that these
    /// files are created on.
    ///
    pub sticky_task_queue: String
}

/// Activity for downloading a file from S3.
/// 
pub async fn create_workspace(_ctx: ActContext, _: CreateWorkspaceInput) -> anyhow::Result<CreateWorkspaceOutput> {
    Ok(CreateWorkspaceOutput {
        root_path: tempfile::NamedTempFile::new()?.into_temp_path().to_path_buf(),
        directory: tempfile::TempDir::new()?.into_path(),
        sticky_task_queue: hostname().to_string()
    })
}