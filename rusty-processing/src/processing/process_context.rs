use std::fmt::Debug;
use anyhow::anyhow;

use futures::Stream;
use tokio::sync::mpsc::Sender;
use tokio_stream::wrappers::ReceiverStream;

use crate::processing::process_output::ProcessOutput;
use crate::processing::ProcessType;

/// Structure defining the context for a processing operation.
///
#[derive(Debug, Clone)]
pub struct ProcessContext {
    /// The MIME type of the file to process.
    ///
    pub mimetype: String,

    /// The types of output to generate.
    ///
    pub types: Vec<ProcessType>,

    /// A sender to send processing results
    ///
    output_sink: Sender<anyhow::Result<ProcessOutput>>,
}

impl ProcessContext {
    pub fn new(
        mimetype: impl Into<String>,
        types: Vec<ProcessType>
    ) -> (Self, impl Stream<Item=anyhow::Result<ProcessOutput>> + Send + Sync + Unpin) {
        let (output_sink, output_stream) = tokio::sync::mpsc::channel(100);
        let output_stream = Box::new(ReceiverStream::new(output_stream));
        let context = Self {
            mimetype: mimetype.into(),
            types,
            output_sink,
        };

        (context, output_stream)
    }

    pub async fn add_result(&self, result: anyhow::Result<ProcessOutput>) -> anyhow::Result<()> {
        self.output_sink.send(result).await
            .map_err(|e| anyhow!(e))
    }
}
