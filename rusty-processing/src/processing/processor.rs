use std::path;

use anyhow::anyhow;
use bytes::Bytes;
use futures::{Stream, StreamExt, TryStreamExt};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio::try_join;

use crate::application::mbox::processor::MboxProcessor;
use crate::message::rfc822::processor::Rfc822Processor;
use crate::processing::{ProcessContext, ProcessType};
use crate::processing::process::Process;
use crate::processing::process_output::ProcessOutput;

lazy_static! {
    static ref PROCESSOR: Processor = Processor;
}

/// Returns a reference to the global processor instance.
///
pub fn processor() -> &'static Processor {
    &PROCESSOR
}

/// Structure defining the core processor.
///
/// The processor is the core processor of the library and is responsible for
/// determining the correct processor to use for a given MIME type, and then
/// delegating to that processor.
///
#[derive(Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct Processor;

impl Processor {
    pub fn process_file<F>(
        &self,
        source_file: path::PathBuf,
        output_dir: path::PathBuf,
        mimetype: String,
        types: Vec<ProcessType>,
    ) -> anyhow::Result<()>
        where F: FnMut(anyhow::Result<ProcessOutput>) + Send + Sync,
    {
        Ok(())
    }

    /// Processes a file.
    ///
    /// This method will determine the correct processor to use for the given
    /// MIME type, and then delegate to that processor.
    ///
    /// # Arguments
    ///
    /// * `source_file` - The path to the file to process.
    /// * `output_dir` - The path to the directory to write output files to.
    /// * `mimetype` - The MIME type of the file to process.
    /// * `types` - The types of output to generate.
    ///
    pub async fn process<'a>(
        &self,
        source_stream: impl Stream<Item=Bytes> + Send + Sync + Unpin + 'a,
        output_sink: Sender<anyhow::Result<ProcessOutput>>,
        mimetype: String,
        types: Vec<ProcessType>,
    ) -> anyhow::Result<()> {
        println!("Processing");

        let (context, output_stream) = ProcessContext::new(
            mimetype,
            types,
        );

        let process_fut = Self::process_mimetype(source_stream, context);
        let transfer_fut = Self::transfer_output(output_stream, output_sink);

        try_join!(process_fut, transfer_fut)?;
        Ok(())
    }

    async fn process_mimetype(
        source_stream: impl Stream<Item=Bytes> + Send + Sync + Unpin,
        context: ProcessContext,
    ) -> anyhow::Result<()> {
        println!("Processing MIME type: {}", context.mimetype);

        let mimetype = context.mimetype.as_ref();
        let processor = match mimetype {
            "application/mbox" => Ok(MboxProcessor),
            // "message/rfc822" => Ok(Rfc822Processor),
            _ => Err(anyhow!("Unsupported MIME type: {}", mimetype)),
        }?;

        processor.process(Box::new(source_stream), context).await
    }

    async fn transfer_output<'a>(
        mut stream: impl Stream<Item=anyhow::Result<ProcessOutput>> + Send + Sync + Unpin,
        sink: Sender<anyhow::Result<ProcessOutput>>,
    ) -> anyhow::Result<()> {
        while let Some(output) = stream.next().await {
            sink.send(output).await
                .map_err(|e| anyhow!(e.to_string()))?;
        }

        Ok(())
    }
}
