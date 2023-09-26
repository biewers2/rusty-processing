use anyhow::anyhow;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::common::ByteStream;
use crate::processing::ProcessContext;
use crate::processing::process::Process;

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
        ctx: ProcessContext,
        stream: ByteStream,
    ) -> anyhow::Result<()> {
        self.processor(&ctx.mimetype)?.process(ctx, stream).await
    }

    fn processor(&self, mimetype: &str) -> anyhow::Result<Box<dyn Process>> {
        match mimetype {
            #[cfg(feature = "mail")]
            "application/mbox" => Ok(Box::<crate::application::mbox::MboxProcessor>::default()),

            #[cfg(feature = "mail")]
            "message/rfc822" => Ok(Box::<crate::message::rfc822::Rfc822Processor>::default()),

            _ => Err(anyhow!("Unsupported MIME type: {}", mimetype)),
        }
    }
}
