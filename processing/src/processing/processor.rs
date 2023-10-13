use std::fmt::{Debug, Display, Formatter};
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use futures::future::try_join_all;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use tempfile::TempPath;

use identify::deduplication::dedupe_checksum_from_path;

use crate::build_paths_from_types;
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
pub(crate) trait Process: Send + Sync {
    /// Process a stream of bytes.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of the processing operation.
    /// * `input_path` - The path to the input file.
    /// * `output_path` - The path to the output file.
    ///
    async fn process(
        &self,
        ctx: &ProcessContext,
        input_path: &Path,
        output_path: Option<TempPath>,
        checksum: &str,
    ) -> anyhow::Result<()>;

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
        input_path: PathBuf,
    ) -> Result<(), ProcessingError> {
        let checksum = dedupe_checksum_from_path(&input_path, &ctx.mimetype).await
            .map_err(|err| ProcessingError::Unexpected(anyhow::Error::from(err)))?;

        let paths = build_paths_from_types(&ctx.types)
            .map_err(|err| ProcessingError::Unexpected(anyhow::Error::from(err)))?;

        let mut futures = vec![];
        let processes = vec![
            (self.text_processor(&ctx.mimetype), paths.text),
            (self.metadata_processor(&ctx.mimetype), paths.metadata),
            (self.pdf_processor(&ctx.mimetype), paths.pdf),
            (self.embedded_processor(&ctx.mimetype), None),
        ];

        for (processor, path) in processes {
            if let Some(processor) = processor {
                let ctx_ref = &ctx;
                let input_path_ref = &input_path;
                let checksum = &checksum;

                futures.push(async move {
                    processor.process(ctx_ref, input_path_ref, path, checksum).await
                });
            }
        }

        try_join_all(futures).await.map_err(|err| ProcessingError::Unexpected(err))?;
        Ok(())
    }

    fn text_processor(&self, mimetype: &str) -> Option<Box<dyn Process>> {
        match mimetype {
            "application/zip" |
            "application/mbox" => None,

            _ => Some(Box::<crate::text::DefaultTextProcessor>::default()),
        }
    }

    fn metadata_processor(&self, mimetype: &str) -> Option<Box<dyn Process>> {
        match mimetype {
            _ => Some(Box::<crate::metadata::DefaultMetadataProcessor>::default())
        }
    }

    fn pdf_processor(&self, mimetype: &str) -> Option<Box<dyn Process>> {
        match mimetype {
            "message/rfc822" => Some(Box::<crate::pdf::Rfc822PdfProcessor>::default()),

            _ => None
        }
    }

    fn embedded_processor(&self, mimetype: &str) -> Option<Box<dyn Process>> {
        match mimetype {
            "application/zip" => Some(Box::<crate::embedded::ZipEmbeddedProcessor>::default()),
            "application/mbox" => Some(Box::<crate::embedded::MboxEmbeddedProcessor>::default()),
            "message/rfc822" => Some(Box::<crate::embedded::Rfc822EmbeddedProcessor>::default()),

            _ => None
        }
    }
}
