use std::fs::File;
use std::io::Read;

use anyhow::anyhow;
use async_trait::async_trait;
use futures::try_join;
use mail_parser::{Message, MessageParser, MimeHeaders};
use serde::{Deserialize, Serialize};

use rusty_processing_identify::identifier;

use crate::common::ByteStream;
use crate::common::workspace::Workspace;
use crate::message::rfc822::mimetype;
use crate::processing::{Process, ProcessContext, ProcessOutput};
use crate::stream_io::stream_to_read;

#[derive(Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct Rfc822Processor;

impl Rfc822Processor {
    /// Processes a message by extracting text and metadata, rendering a PDF, and then finding any embedded attachments.
    ///
    async fn process(&self, ctx: ProcessContext, message: Message<'_>) -> anyhow::Result<()> {
        let dedupe_id = identifier(&ctx.mimetype).identify(message.raw_message());
        let wkspace = Workspace::new(&message.raw_message, &ctx.types)?;

        let text_fut = self.process_text(&ctx, &message, wkspace.text_path, &dedupe_id);
        let meta_fut = self.process_metadata(&ctx, &message, wkspace.metadata_path, &dedupe_id);
        let pdf_fut = self.process_pdf(&ctx, &message, wkspace.pdf_path, &dedupe_id);
        let attach_fut = self.process_attachments(&ctx, &message);

        try_join!(text_fut, meta_fut, pdf_fut, attach_fut)?;
        Ok(())
    }

    /// Extracts the text from the message and emits it as processed output.
    ///
    async fn process_text(
        &self,
        ctx: &ProcessContext,
        message: &Message<'_>,
        path: Option<tempfile::TempPath>,
        dedupe_id: impl Into<String>,
    ) -> anyhow::Result<()> {
        if let Some(path) = path {
            let mut writer = File::create(&path)?;
            let result = self.extract_text(message, &mut writer).map(|_|
                ProcessOutput::processed(ctx, "extracted.txt", path, "text/plain", dedupe_id)
            );
            ctx.add_output(result).await?;
        }

        Ok(())
    }

    /// Extracts the metadata from the message and emits it as processed output.
    ///
    async fn process_metadata(
        &self,
        ctx: &ProcessContext,
        message: &Message<'_>,
        path: Option<tempfile::TempPath>,
        dedupe_id: impl Into<String>,
    ) -> anyhow::Result<()> {
        if let Some(path) = path {
            let mut writer = File::create(&path)?;
            let result = self.extract_metadata(message, &mut writer).map(|_|
                ProcessOutput::processed(ctx, "metadata.json", path, "application/json", dedupe_id)
            );
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
        path: Option<tempfile::TempPath>,
        dedupe_id: impl Into<String>,
    ) -> anyhow::Result<()> {
        if let Some(path) = path {
            let mut writer = File::create(&path)?;
            let result = self.render_pdf(message, &mut writer).map(|_|
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
            let dedupe_id = identifier(&mimetype).identify(part.contents());
            let name = part.attachment_name().unwrap_or("message-attachment.dat");
            let Workspace { original_path: original_file, .. } = Workspace::new(part.contents(), &[])?;

            ctx.add_output(Ok(ProcessOutput::embedded(ctx, name, original_file, mimetype, dedupe_id))).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Process for Rfc822Processor {
    async fn process(&self, ctx: ProcessContext, content: ByteStream) -> anyhow::Result<()> {
        let mut raw = Vec::new();
        let mut reader = stream_to_read(content);
        reader.read_to_end(&mut raw)?;

        let parser = MessageParser::default();
        let message = parser.parse(&raw).ok_or(anyhow!("failed to parse message"))?;
        self.process(ctx, message).await?;

        Ok(())
    }
}
