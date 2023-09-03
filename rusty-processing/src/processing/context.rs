use std::path;
use std::sync::{mpsc, Mutex};

use crate::common::output_type::ProcessType;
use crate::processing::output::{Output, OutputInfo};

/// Structure defining the context for a processing operation.
///
#[derive(Debug)]
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
    pub result_tx: Mutex<Option<mpsc::Sender<anyhow::Result<Output>>>>,
}

impl Context {
    pub fn with_mimetype(&self, mimetype: &str) -> Self {
        Self {
            mimetype: mimetype.to_string(),
            ..self.clone()
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
        let tx = self.result_tx.lock().unwrap();
        if let Some(tx) = tx.as_ref() {
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

impl Clone for Context {
    fn clone(&self) -> Self {
        let result_tx = self.result_tx.lock().unwrap().clone();
        Self {
            output_dir: self.output_dir.clone(),
            mimetype: self.mimetype.clone(),
            types: self.types.clone(),
            result_tx: Mutex::new(result_tx),
        }
    }
}