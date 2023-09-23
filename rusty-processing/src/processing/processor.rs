use anyhow::anyhow;
use futures::{Stream, StreamExt};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tokio::try_join;

use crate::application::mbox::processor::MboxProcessor;
use crate::common::ByteStream;
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
    /// Processes a stream of data.
    ///
    /// This method will determine the correct processor to use for the given
    /// MIME type, and then delegate to that processor.
    ///
    /// # Arguments
    ///
    /// * `source_stream` - Stream of data in `bytes::Bytes` of the content to process.
    /// * `output_sink` - The sender used to send the output artifacts from processing. This allows for concurrent handling
    ///     of the output by the caller.
    /// * `mimetype` - The MIME type of the file to process.
    /// * `types` - The types of output to generate.
    ///
    pub async fn process(
        &self,
        source_stream: ByteStream,
        output_sink: Sender<anyhow::Result<ProcessOutput>>,
        mimetype: String,
        types: Vec<ProcessType>,
    ) -> anyhow::Result<()> {
        let (context, output_stream) = ProcessContext::new(
            mimetype,
            types,
        );

        let process_fut = self.process_mimetype(source_stream, context);
        let transfer_fut = self.transfer_output(output_stream, output_sink);

        try_join!(process_fut, transfer_fut)?;
        Ok(())
    }

    async fn process_mimetype(
        &self,
        source_stream: ByteStream,
        context: ProcessContext,
    ) -> anyhow::Result<()> {
        let mimetype = context.mimetype.as_ref();
        self.processor(mimetype)?.process(source_stream, context).await
    }

    async fn transfer_output<'a>(
        &self,
        mut stream: impl Stream<Item=anyhow::Result<ProcessOutput>> + Send + Sync + Unpin,
        sink: Sender<anyhow::Result<ProcessOutput>>,
    ) -> anyhow::Result<()> {
        while let Some(output) = stream.next().await {
            sink.send(output).await
                .map_err(|e| anyhow!(e.to_string()))?;
        }

        Ok(())
    }

    fn processor(&self, mimetype: &str) -> anyhow::Result<Box<dyn Process>> {
        match mimetype {
            "application/mbox" => Ok(Box::<MboxProcessor>::default()),
            "message/rfc822" => Ok(Box::<Rfc822Processor>::default()),
            _ => Err(anyhow!("Unsupported MIME type: {}", mimetype)),
        }
    }
}
