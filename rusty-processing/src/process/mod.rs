mod processor;
mod process_context;
mod process_output;
mod process;

use std::str::FromStr;
use serde::{Deserialize, Serialize};

pub use self::processor::*;
pub use self::process_output::*;
pub(crate) use self::process_context::*;
pub(crate) use self::process::*;

/// The type of output to produce from process.
///
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum ProcessType {
    /// Extracted text of a file.
    ///
    Text,

    /// Metadata of a file.
    ///
    Metadata,

    /// A rendered version of a file as a PDF.
    ///
    Pdf,
}

impl FromStr for ProcessType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(ProcessType::Text),
            "metadata" => Ok(ProcessType::Metadata),
            "pdf" => Ok(ProcessType::Pdf),
            _ => Err(format!("Can not convert {} to OutputType", s)),
        }
    }
}