use std::fs::File;
use std::io::Read;
use std::path;

use anyhow::anyhow;
use async_trait::async_trait;
use futures::try_join;
use mail_parser::{Message, MessageParser, MimeHeaders};
use serde::{Deserialize, Serialize};

use crate::common::{ByteStream, mimetype, StreamReader};
use crate::common::workspace::Workspace;
use crate::processing::{Process, ProcessContext, ProcessOutput, ProcessOutputType};

#[derive(Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct Rfc822Processor;

impl Rfc822Processor {
    /// Processes a message by extracting text and metadata, rendering a PDF, and then finding any embedded attachments.
    ///
    async fn process(&self, message: Message<'_>, context: &ProcessContext) -> anyhow::Result<()> {
        let wkspace = Workspace::new(
            &message.raw_message,
            &context.mimetype,
            &context.types
        )?;

        let text_fut = self.process_text(&message, wkspace.text_path, &wkspace.dupe_id, context);
        let meta_fut = self.process_metadata(&message, wkspace.metadata_path, &wkspace.dupe_id, context);
        let pdf_fut = self.process_pdf(&message, wkspace.pdf_path, &wkspace.dupe_id, context);
        let attach_fut = self.process_attachments(&message, context);

        try_join!(text_fut, meta_fut, pdf_fut, attach_fut)?;
        Ok(())
    }

    /// Extracts the text from the message and emits it as processed output.
    ///
    async fn process_text(
        &self,
        message: &Message<'_>,
        path: Option<impl AsRef<path::Path>>,
        dupe_id: impl Into<String>,
        context: &ProcessContext,
    ) -> anyhow::Result<()> {
        if let Some(path) = path {
            let mut writer = File::create(&path)?;
            let result = self.extract_text(message, &mut writer).map(|_| {
                ProcessOutput {
                    path: path.as_ref().to_path_buf(),
                    output_type: ProcessOutputType::Processed,
                    mimetype: "text/plain".to_string(),
                    dupe_id: dupe_id.into(),
                }
            });

            context.add_result(result).await?;
        }

        Ok(())
    }

    /// Extracts the metadata from the message and emits it as processed output.
    ///
    async fn process_metadata(
        &self,
        message: &Message<'_>,
        path: Option<impl AsRef<path::Path>>,
        dupe_id: impl Into<String>,
        context: &ProcessContext,
    ) -> anyhow::Result<()> {
        if let Some(path) = path {
            let mut writer = File::create(&path)?;
            let result = self.extract_metadata(message, &mut writer).map(|_| {
                ProcessOutput {
                    path: path.as_ref().to_path_buf(),
                    output_type: ProcessOutputType::Processed,
                    mimetype: "application/json".to_string(),
                    dupe_id: dupe_id.into(),
                }
            });

            context.add_result(result).await?;
        }

        Ok(())
    }

    /// Renders a PDF from the message and emits it as processed output.
    ///
    async fn process_pdf(
        &self,
        message: &Message<'_>,
        path: Option<impl AsRef<path::Path>>,
        dupe_id: impl Into<String>,
        context: &ProcessContext,
    ) -> anyhow::Result<()> {
        if let Some(path) = path {
            let mut writer = File::create(&path)?;
            let result = self.render_pdf(message, &mut writer).map(|_| {
                ProcessOutput {
                    path: path.as_ref().to_path_buf(),
                    output_type: ProcessOutputType::Processed,
                    mimetype: "application/pdf".to_string(),
                    dupe_id: dupe_id.into(),
                }
            });

            context.add_result(result).await?;
        }

        Ok(())
    }

    /// Discovers any attachments in the message and emits them as embedded output.
    ///
    async fn process_attachments(&self, message: &Message<'_>, context: &ProcessContext) -> anyhow::Result<()> {
        for part_id in &message.attachments {
            let part = message
                .part(*part_id)
                .ok_or(anyhow!("failed to get attachment part"))?;
            let content_type = part
                .content_type()
                .ok_or(anyhow!("failed to get attachment content type"))?;
            let mimetype = mimetype(content_type);
            let Workspace { original_path, dupe_id, .. } = Workspace::new(
                part.contents(),
                &mimetype,
                &[]
            )?;

            context.add_result(Ok(ProcessOutput {
                path: original_path,
                output_type: ProcessOutputType::Embedded,
                mimetype,
                dupe_id,
            })).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Process for Rfc822Processor {
    async fn process(&self, content: ByteStream, context: ProcessContext) -> anyhow::Result<()> {
        let mut raw = Vec::new();
        StreamReader::new(Box::new(content)).read_to_end(&mut raw)?;

        let parser = MessageParser::default();
        let message = parser.parse(&raw).ok_or(anyhow!("failed to parse message"))?;
        self.process(message, &context).await?;

        Ok(())
    }
}
