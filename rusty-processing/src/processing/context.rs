use std::path;
use std::sync::mpsc;

use crate::common::output_type::ProcessType;
use crate::processing::output::{Output, OutputInfo};

/// Structure defining the context for a processing operation.
///
#[derive(Clone, Debug)]
pub struct Context {
    /// The path to the directory to write output files to.
    ///
    pub output_dir: path::PathBuf,

    /// The MIME type of the file to process.
    ///
    pub mimetype: String,

    /// The types of output to generate.
    ///
    pub types: Vec<ProcessType>,

    /// The channel to send the result of the processing operation to.
    ///
    pub result_tx: Option<mpsc::Sender<anyhow::Result<Output>>>,
}

impl Context {
    pub fn with_mimetype(&self, mimetype: &str) -> Self {
        Self {
            output_dir: self.output_dir.clone(),
            mimetype: mimetype.to_string(),
            types: self.types.clone(),
            result_tx: self.result_tx.clone(),
        }
    }

    /// Sends the result of the processing operation to the result channel.
    ///
    /// Ignores errors occurred during sending as the assumption is this will be run from a thread.
    ///
    /// # Arguments
    ///
    /// * `result` - The result of the processing operation.
    ///
    pub fn send_result(&self, result: anyhow::Result<Output>) {
        if let Some(tx) = &self.result_tx {
            tx.send(result).unwrap_or(());
        }
    }

    /// Determines whether the given output type should be processed.
    ///
    /// # Arguments
    ///
    /// * `output_type` - The output type to check.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the given output type should be processed.
    ///
    pub fn should_process_type(&self, output_type: &ProcessType) -> bool {
        self.types.contains(output_type)
    }
}
