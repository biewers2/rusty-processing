use std::fmt::Debug;
use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use async_trait::async_trait;
use futures::try_join;
use mail_parser::{Message, MessageParser, MimeHeaders};
use tempfile::NamedTempFile;

use identify::deduplication::{dedupe_checksum, dedupe_checksum_from_path};
use services::tika;

use crate::build_paths_from_types;
use crate::message::rfc822::mimetype;
use crate::processing::{Process, ProcessContext, ProcessOutput};

#[derive(Debug, Default)]
pub struct Rfc822Processor {
    message_parser: MessageParser,
}

#[async_trait]
impl Process for Rfc822Processor {
    /// Processes a message by extracting text and metadata, rendering a PDF, and then finding any embedded attachments.
    ///
    async fn process(&self, ctx: ProcessContext, path: PathBuf) -> anyhow::Result<()> {
        let checksum = dedupe_checksum_from_path(&path, &ctx.mimetype).await?;
        let paths = build_paths_from_types(&ctx.types)?;

        let text_fut = self.process_text(&ctx, &path, paths.text, &checksum);
        let meta_fut = self.process_metadata(&ctx, &path, paths.metadata, &checksum);

        let content = Box::new(tokio::fs::read(&path).await?);
        let message = self.message_parser.parse(content.as_ref()).ok_or(anyhow!("failed to parse message"))?;
        let pdf_fut = self.process_pdf(&ctx, &message, paths.pdf, &checksum);
        let attach_fut = self.process_attachments(&ctx, &message);

        try_join!(text_fut, meta_fut, pdf_fut, attach_fut)?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "RFC 822"
    }
}

impl Rfc822Processor {
    /// Extracts the text from the message and emits it as processed output.
    ///
    async fn process_text(
        &self,
        ctx: &ProcessContext,
        message_path: impl AsRef<Path>,
        text_path: Option<tempfile::TempPath>,
        dedupe_id: impl Into<String>,
    ) -> anyhow::Result<()> {
        if let Some(path) = text_path {
            tika().text_into_file(message_path, &path).await?;

            let output = ProcessOutput::processed(ctx, "extracted.txt", path, "text/plain", dedupe_id);
            ctx.add_output(Ok(output)).await?;
        }

        Ok(())
    }

    /// Extracts the metadata from the message and emits it as processed output.
    ///
    async fn process_metadata(
        &self,
        ctx: &ProcessContext,
        message_path: impl AsRef<Path>,
        metadata_path: Option<tempfile::TempPath>,
        dedupe_id: impl Into<String>,
    ) -> anyhow::Result<()> {
        if let Some(path) = metadata_path {
            let result = async {
                let mut metadata = tika().metadata(message_path).await?;
                tokio::fs::write(&path, &mut metadata).await?;

                let output = ProcessOutput::processed(ctx, "metadata.json", path, "application/json", dedupe_id);
                anyhow::Ok(output)
            }.await;

            ctx.add_output(result).await?;
        }
        Ok(())
    }

    /// Renders a PDF from the message and emits it as processed output.
    ///
    async fn process_pdf(
        &self,
        ctx: &ProcessContext,
        message: &Message<'_>,
        pdf_path: Option<tempfile::TempPath>,
        dedupe_id: impl Into<String>,
    ) -> anyhow::Result<()> {
        if let Some(path) = pdf_path {
            let mut writer = File::create(&path)?;
            let result = self.render_pdf(message, &mut writer).await.map(|_|
                ProcessOutput::processed(ctx, "rendered.pdf", path, "application/pdf", dedupe_id)
            );
            ctx.add_output(result).await?;
        }
        Ok(())
    }

    /// Discovers any attachments in the message and emits them as embedded output.
    ///
    async fn process_attachments(&self, ctx: &ProcessContext, message: &Message<'_>) -> anyhow::Result<()> {
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

            let output = ProcessOutput::embedded(ctx, name, file.into_temp_path(), mimetype, checksum);
            ctx.add_output(Ok(output)).await?;
        }

        Ok(())
    }
}
