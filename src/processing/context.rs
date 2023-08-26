use crate::common::error::ProcessResult;
use crate::common::output_type::OutputType;
use std::path;
use std::sync::mpsc;

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
    pub types: Option<Vec<OutputType>>,

    /// The channel to send the result of the processing operation to.
    ///
    pub result_tx: Option<mpsc::Sender<ProcessResult<()>>>,
}

impl Context {
    /// Sends the result of the processing operation to the result channel.
    ///
    /// Ignores errors occurred during sending as the assumption is this will be run from a thread.
    ///
    /// # Arguments
    ///
    /// * `result` - The result of the processing operation.
    ///
    pub fn send_result(&self, result: ProcessResult<()>) {
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
    pub fn should_process_type(&self, output_type: &OutputType) -> bool {
        self.types
            .as_ref()
            .map_or(true, |types| types.contains(output_type))
    }
}
