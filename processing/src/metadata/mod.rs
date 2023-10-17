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
        ctx: ProcessContext,
        input_path: &Path,
        output_path: TempPath,
        checksum: &str,
    ) -> anyhow::Result<()> {
        let result = async {
            let mut metadata = tika().metadata(input_path).await?;
            tokio::fs::write(&output_path, &mut metadata).await?;

            let output = ProcessOutput::processed(&ctx, "metadata.json", output_path, "embedded/json", checksum);
            anyhow::Ok(output)
        }.await;

        ctx.add_output(result).await
    }

    fn name(&self) -> &'static str {
        "Default Metadata"
    }
}