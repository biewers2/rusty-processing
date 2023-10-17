use std::fmt::Debug;
use std::io::Cursor;
use std::path::Path;

use anyhow::anyhow;
use async_trait::async_trait;
use mail_parser::{MessageParser, MimeHeaders};
use tempfile::{NamedTempFile, TempPath};

use identify::deduplication::dedupe_checksum;

use crate::mimetype;
use crate::processing::{Process, ProcessContext, ProcessOutput};

#[derive(Debug, Default)]
pub struct Rfc822EmbeddedProcessor {
    message_parser: MessageParser,
}

#[async_trait]
impl Process for Rfc822EmbeddedProcessor {
    async fn process(
        &self,
        ctx: ProcessContext,
        input_path: &Path,
        _: TempPath,
        _: &str,
    ) -> anyhow::Result<()> {
        let content = std::fs::read(input_path)?;
        let message = self.message_parser.parse(&content)
            .ok_or(anyhow!("Failed to parse message"))?;

        for part_id in &message.attachments {
            let part = message
                .part(*part_id)
                .ok_or(anyhow!("failed to get attachment part"))?;
            let content_type = part
                .content_type()
                .ok_or(anyhow!("failed to get attachment content type"))?;
            let mimetype = mimetype(content_type);

            let mut reader = Cursor::new(part.contents());
            let checksum = dedupe_checksum(&mut reader, &mimetype).await?;
            let name = part.attachment_name().unwrap_or("message-attachment.dat");

            let mut file = NamedTempFile::new()?;
            std::io::copy(&mut part.contents(), &mut file)?;

            let output = ProcessOutput::embedded(&ctx, name, file.into_temp_path(), mimetype, checksum);
            ctx.add_output(Ok(output)).await?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "RFC 822 Embedded"
    }
}
