use std::str::FromStr;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;

pub(crate) use self::process::*;
pub use self::processor::*;

mod processor;
mod process;

/// The type of output to produce from processing.
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

/// Represents the state of a processing operation.
///
/// This is built and modified during processing and is provided with the final processing output.
///
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct ProcessState {
    /// The chain of IDs representing a unique identifying path to the current file being processed.
    ///
    /// For an overall processing operation, this is useful for defining the structure of embedded and processed files stemming from
    /// a root file. This structure is a tree, where the embedded files are branches and the processed files are leaves.
    ///
    pub id_chain: Vec<String>,
}

/// Defines the context for a processing operation.
///
/// This is passed to the root processing function and is used to provide information about the current file being processed,
/// as well as any additional parameters or information that the processing operation may use through its lifetime, such as the `types`.
///
#[derive(Debug, Clone)]
pub struct ProcessContext {
    /// The MIME type of the file to process.
    ///
    pub mimetype: String,

    /// The types of output to generate.
    ///
    pub types: Vec<ProcessType>,

    /// The state of the processing operation.
    ///
    pub state: ProcessState,

    output_sink: Sender<anyhow::Result<ProcessOutput>>,
}

impl ProcessContext {
    /// Creates a new ProcessContext with the given MIME type.
    ///
    /// Clones all other fields from the current ProcessContext.
    ///
    pub fn new_clone(&self, mimetype: String) -> Self {
        Self {
            mimetype,
            types: self.types.clone(),
            output_sink: self.output_sink.clone(),
            state: self.state.clone(),
        }
    }

    /// Adds an output to be sent through the output transfer channel created by the caller of the processing operation.
    ///
    pub async fn add_output(&self, result: anyhow::Result<ProcessOutput>) -> anyhow::Result<()> {
        self.output_sink.send(result).await
            .map_err(|e| anyhow!(e))
    }

    /// Returns the current ID chain.
    ///
    /// See `ProcessState.id_chain` for more information.
    ///
    pub fn id_chain(self) -> Vec<String> {
        self.state.id_chain
    }
}

/// Builder for ProcessContext.
///
#[derive(Debug, Clone)]
pub struct ProcessContextBuilder {
    mimetype: String,
    types: Vec<ProcessType>,
    output_sink: Sender<anyhow::Result<ProcessOutput>>,
    state: ProcessState,
}

impl ProcessContextBuilder {
    /// Creates a new ProcessContextBuilder with the given MIME type, types of files to process, and output transfer channel.
    ///
    /// # Arguments
    ///
    /// * `mimetype` - The MIME type of the file to process.
    /// * `types` - The types of output to generate.
    /// * `output_sink` - The output transfer channel created by the caller of the processing operation.
    ///
    pub fn new(
        mimetype: impl Into<String>,
        types: Vec<ProcessType>,
        output_sink: Sender<anyhow::Result<ProcessOutput>>,
    ) -> Self {
        ProcessContextBuilder {
            mimetype: mimetype.into(),
            types,
            output_sink,
            state: ProcessState {
                id_chain: Vec::new(),
            }
        }
    }

    /// Set the MIME type to use.
    ///
    pub fn mimetype(mut self, mimetype: impl Into<String>) -> Self {
        self.mimetype = mimetype.into();
        self
    }

    /// Set the types of output to generate.
    ///
    pub fn types(mut self, types: Vec<ProcessType>) -> Self {
        self.types = types;
        self
    }

    /// Sets the ID chain.
    ///
    /// See `ProcessState.id_chain` for more information.
    ///
    pub fn id_chain(mut self, id_chain: Vec<String>) -> Self {
        self.state.id_chain = id_chain;
        self
    }

    /// Build the ProcessContext.
    ///
    pub fn build(self) -> ProcessContext {
        ProcessContext {
            mimetype: self.mimetype,
            types: self.types,
            output_sink: self.output_sink,
            state: self.state,
        }
    }
}

impl From<ProcessContext> for ProcessContextBuilder {
    fn from(context: ProcessContext) -> Self {
        ProcessContextBuilder {
            mimetype: context.mimetype,
            types: context.types,
            output_sink: context.output_sink,
            state: context.state,
        }
    }
}

/// Representation of the output of processing a file.
///
/// It can be either a new file or an embedded file.
///
#[derive(Debug)]
pub enum ProcessOutput {
    /// A newly created file as a result of processing the original file.
    ///
    Processed(ProcessState, ProcessOutputData),

    /// A file discovered during the processing of the original file.
    ///
    Embedded(ProcessState, ProcessOutputData, Sender<anyhow::Result<ProcessOutput>>),
}

/// Data associated with the file created.
///
/// It contains the path, mimetype and the deduplication identifier.
///
#[derive(Debug)]
pub struct ProcessOutputData {
    /// The name to give the output file.
    ///
    pub name: String,

    /// The output file.
    ///
    pub path: tempfile::TempPath,

    /// Mimetype of the output file.
    ///
    pub mimetype: String,

    /// The types of output generated.
    ///
    pub types: Vec<ProcessType>,

    /// Deduplication ID of the output file.
    ///
    pub dedupe_id: String,
}

impl ProcessOutput {
    /// Creates a new ProcessOutput representing a newly created file.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The ProcessContext of the processing operation.
    /// * `path` - The path to the output file.
    /// * `mimetype` - The MIME type of the output file.
    /// * `dedupe_id` - The dupe ID of the output file.
    ///
    pub fn processed(
        ctx: &ProcessContext,
        name: impl Into<String>,
        path: tempfile::TempPath,
        mimetype: impl Into<String>,
        dedupe_id: impl Into<String>,
    ) -> Self {
        Self::Processed(
            ctx.state.clone(),
            ProcessOutputData {
                name: name.into(),
                path,
                mimetype: mimetype.into(),
                types: ctx.types.clone(),
                dedupe_id: dedupe_id.into(),
            }
        )
    }

    /// Creates a new ProcessOutput representing an embedded file.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The ProcessContext of the processing operation.
    /// * `path` - The path to the output file.
    /// * `mimetype` - The MIME type of the output file.
    /// * `dedupe_id` - The dupe ID of the output file.
    ///
    pub fn embedded(
        ctx: &ProcessContext,
        name: impl Into<String>,
        path: tempfile::TempPath,
        mimetype: impl Into<String>,
        dedupe_id: impl Into<String>,
    ) -> Self {
        Self::Embedded(
            ctx.state.clone(),
            ProcessOutputData {
                name: name.into(),
                path,
                mimetype: mimetype.into(),
                types: ctx.types.clone(),
                dedupe_id: dedupe_id.into(),
            },
            ctx.output_sink.clone(),
        )
    }
}