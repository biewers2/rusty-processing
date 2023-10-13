use std::path::Path;
use async_trait::async_trait;
use tempfile::TempPath;
use services::tika;
use crate::processing::{Process, ProcessContext, ProcessOutput};

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefaultMetadataProcessor;

#[async_trait]
impl Process for DefaultMetadataProcessor {
    async fn process(
        &self,
        ctx: &ProcessContext,
        input_path: &Path,
        output_path: Option<TempPath>,
        checksum: &str,
    ) -> anyhow::Result<()> {
        if let Some(path) = output_path {
            let result = async {
                let mut metadata = tika().metadata(input_path).await?;
                tokio::fs::write(&path, &mut metadata).await?;

                let output = ProcessOutput::processed(ctx, "metadata.json", path, "embedded/json", checksum);
                anyhow::Ok(output)
            }.await;

            ctx.add_output(result).await?;
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "Default Metadata"
    }
}