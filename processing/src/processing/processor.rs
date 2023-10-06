use anyhow::anyhow;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use crate::io::ByteStream;

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
    /// * `ctx` - Context of the processing operation.
    /// * `stream` - Stream of data in `bytes::Bytes` of the content to process.
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
            #[cfg(feature = "archive")]
            "application/zip" => Ok(Box::<crate::application::zip::ZipProcessor>::default()),

            #[cfg(feature = "mail")]
            "application/mbox" => Ok(Box::<crate::application::mbox::MboxProcessor>::default()),

            #[cfg(feature = "mail")]
            "message/rfc822" => Ok(Box::<crate::message::rfc822::Rfc822Processor>::default()),

            _ => Err(anyhow!("Unsupported MIME type: {}", mimetype)),
        }
    }
}
