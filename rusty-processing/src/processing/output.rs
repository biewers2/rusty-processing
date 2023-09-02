use serde::{Deserialize, Serialize};
use std::path;

pub enum Output {
    Processed(OutputInfo),
    Embedded(OutputInfo),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OutputInfo {
    pub path: path::PathBuf,
    pub mimetype: String,
    pub dupe_id: String,
}
