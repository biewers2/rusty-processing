use std::fmt::Debug;
use std::fs::File;
use std::path::Path;
use anyhow::anyhow;

use async_trait::async_trait;
use mail_parser::MessageParser;
use tempfile::TempPath;

use crate::processing::{Process, ProcessContext, ProcessOutput};

mod html_message_visitor;
mod message_formatter;
mod message_visitor;
mod transformer;

mod pdf;

#[derive(Debug, Default)]
pub struct Rfc822PdfProcessor {
    message_parser: MessageParser,
}

#[async_trait]
impl Process for Rfc822PdfProcessor {
    async fn process(
        &self,
        ctx: ProcessContext,
        input_path: &Path,
        output_path: TempPath,
        checksum: &str,
    ) -> anyhow::Result<()> {
        let content = std::fs::read(input_path)?;
        let message = self.message_parser.parse(&content)
            .ok_or(anyhow!("Failed to parse message"))?;

        let mut writer = File::create(&output_path)?;
        let result = self.render_pdf(&message, &mut writer).await.map(|_|
            ProcessOutput::processed(&ctx, "rendered.pdf", output_path, "embedded/pdf", checksum)
        );
        ctx.add_output(result).await
    }


    fn name(&self) -> &'static str {
        "RFC 822 PDF"
    }
}
