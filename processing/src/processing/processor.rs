use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;
use async_trait::async_trait;
use lazy_static::lazy_static;
use log::info;
use serde::{Deserialize, Serialize};

use crate::processing::ProcessContext;

lazy_static! {
    static ref PROCESSOR: Processor = Processor;
}

/// Returns a reference to the global processor instance.
///
pub fn processor() -> &'static Processor {
    &PROCESSOR
}

/// Error that can occur during processing.
///
#[derive(Debug)]
pub enum ProcessingError {
    /// The MIME type is not supported by the processor.
    ///
    UnsupportedMimeType(String),

    /// An unexpected error occurred.
    ///
    Unexpected(anyhow::Error),
}

impl Display for ProcessingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedMimeType(mimetype) => write!(f, "Unsupported MIME type: {}", mimetype),
            Self::Unexpected(err) => write!(f, "Unexpected error: {}", err),
        }
    }
}

/// Process is a trait that defines the interface for process data from a file or as raw bytes.
///
/// Process implementations are required to be thread safe.
///
#[async_trait]
pub trait Process: Send + Sync {
    /// Process a stream of bytes.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of the processing operation.
    /// * `content` - Async reader of the raw bytes to process.
    ///
    async fn process(&self, ctx: ProcessContext, path: PathBuf) -> anyhow::Result<()>;

    /// Returns the name of the processor.
    ///
    fn name(&self) -> &'static str;
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
        path: PathBuf,
    ) -> Result<(), ProcessingError> {
        let processor = self.processor(&ctx.mimetype)?;

        info!("Processing {} with processor {}", ctx.mimetype, processor.name());
        processor.process(ctx, path).await
            .map_err(|err| ProcessingError::Unexpected(err))
    }

    fn processor(&self, mimetype: &str) -> Result<Box<dyn Process>, ProcessingError> {
        match mimetype {
            #[cfg(feature = "archive")]
            "application/zip" => Ok(Box::<crate::application::zip::ZipProcessor>::default()),

            #[cfg(feature = "mail")]
            "application/mbox" => Ok(Box::<crate::application::mbox::MboxProcessor>::default()),

            #[cfg(feature = "mail")]
            "message/rfc822" => Ok(Box::<crate::message::rfc822::Rfc822Processor>::default()),

            _ => Err(ProcessingError::UnsupportedMimeType(mimetype.to_string())),
        }
    }
}
