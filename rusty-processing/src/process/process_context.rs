use std::fmt::Debug;
use std::path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use crate::process::process_output::ProcessOutput;
use crate::process::ProcessType;

/// Structure defining the context for a process operation.
///
#[derive(Debug, Clone)]
pub struct ProcessContext {
    /// The path to the directory to write output files to.
    ///
    pub output_dir: path::PathBuf,

    /// The MIME type of the file to process.
    ///
    pub mimetype: String,

    /// The types of output to generate.
    ///
    pub types: Vec<ProcessType>,

    /// A sender to send processing results
    ///
    result_tx: Sender<anyhow::Result<ProcessOutput>>,
}

impl ProcessContext {
    pub fn new(
        output_dir: path::PathBuf,
        mimetype: impl Into<String>,
        types: Vec<ProcessType>,
    ) -> (Self, Receiver<anyhow::Result<ProcessOutput>>) {
        let (tx, rx) = mpsc::channel();
        let context = Self {
            output_dir,
            mimetype: mimetype.into(),
            types,
            result_tx: tx,
        };

        (context, rx)
    }

    pub fn with_mimetype(self, mimetype: impl Into<String>) -> Self {
        Self {
            mimetype: mimetype.into(),
            ..self
        }
    }

    pub fn add_result(&self, result: anyhow::Result<ProcessOutput>) {
        self.result_tx.send(result).unwrap_or(());
    }
}
