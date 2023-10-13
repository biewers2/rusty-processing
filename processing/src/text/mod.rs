use std::path::Path;

use async_trait::async_trait;
use tempfile::TempPath;

use services::tika;

use crate::processing::{Process, ProcessContext, ProcessOutput};

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefaultTextProcessor;

#[async_trait]
impl Process for DefaultTextProcessor {
    async fn process(
        &self,
        ctx: &ProcessContext,
        input_path: &Path,
        output_path: Option<TempPath>,
        checksum: &str,
    ) -> anyhow::Result<()> {
        if let Some(path) = output_path {
            tika().text_into_file(input_path, &path).await?;

            let output = ProcessOutput::processed(ctx, "extracted.txt", path, "text/plain", checksum);
            ctx.add_output(Ok(output)).await?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "Default Text"
    }
}