use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use temporal_sdk::ActContext;

/// Input to the `create_workspace` activity.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveWorkspaceInput {
    /// The local paths to remove from the machine running the activity.
    ///
    pub paths: Vec<String>
}

/// Activity for removing files from the local machine.
///
pub async fn remove_workspace(_ctx: ActContext, input: RemoveWorkspaceInput) -> anyhow::Result<()> {
    input.paths.into_iter()
        .filter(|path| path.starts_with("/tmp"))
        .map(|path| PathBuf::from(path))
        .try_for_each(|path| {
            if path.is_dir() {
                std::fs::remove_dir_all(path)
            } else {
                std::fs::remove_file(path)
            }
        })?;
    Ok(())
}