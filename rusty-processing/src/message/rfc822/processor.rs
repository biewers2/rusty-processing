use std::{future, path, thread};
use std::fs::File;
use std::io::Read;
use std::ops::Deref;
use std::pin::Pin;

use anyhow::anyhow;
use mail_parser::{Message, MimeHeaders};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncRead;

use crate::common::util::mimetype;
use crate::common::workspace::Workspace;
use crate::process::{Process, ProcessContext, ProcessOutput, ProcessOutputType};

#[derive(Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct Rfc822Processor;

impl Rfc822Processor {
    /// Processes a message by extracting text and metadata, rendering a PDF, and then finding any embedded attachments.
    ///
    fn process(&self, message: Message<'_>, context: ProcessContext) -> anyhow::Result<()> {
        let wkspace = Workspace::new(
            &message.raw_message,
            &context.output_dir,
            &context.mimetype,
            &context.types
        ).await?;

        let text_processing = wkspace.text_path.as_ref()
            .map(|path| self.process_text(&message, path, &wkspace.dupe_id, &context));
        let metadata_processing = wkspace.metadata_path.as_ref()
            .map(|path| self.process_metadata(&message, path, &wkspace.dupe_id, &context));
        let pdf_path = wkspace.pdf_path.as_ref()
            .map(|path| self.process_pdf(&message, path, &wkspace.dupe_id, &context));

        self.process_attachments(&message, &context).await?;

        if let Some(process) = text_processing {
            process.await?;
        }
        if let Some(process) = metadata_processing {
            process.await?;
        }
        if let Some(process) = pdf_path {
            process.await?;
        }

        Ok(())
    }

    /// Extracts the text from the message and emits it as processed output.
    ///
    fn process_text<'a>(
        &self,
        message: &Message<'a>,
        path: impl AsRef<path::Path>,
        dupe_id: impl Into<String>,
        context: &ProcessContext,
    ) -> anyhow::Result<()> {
        let mut writer = File::create(&path)?;
        let result = self.extract_text(&message, &mut writer).map(|_| {
            ProcessOutput {
                path: path.as_ref().to_path_buf(),
                output_type: ProcessOutputType::Processed,
                mimetype: "text/plain".to_string(),
                dupe_id: dupe_id.into(),
            }
        });
        context.result_handler.handle(result);

        Ok(())
    }

    /// Extracts the metadata from the message and emits it as processed output.
    ///
    fn process_metadata<'a>(
        &self,
        message: &Message<'a>,
        path: impl AsRef<path::Path>,
        dupe_id: impl Into<String>,
        context: &ProcessContext,
    ) -> anyhow::Result<()> {
        let mut writer = File::create(&path)?;
        let result = self.extract_metadata(&message, &mut writer).map(|_| {
            ProcessOutput {
                path: path.as_ref().to_path_buf(),
                output_type: ProcessOutputType::Processed,
                mimetype: "application/json".to_string(),
                dupe_id: dupe_id.into(),
            }
        });
        context.result_handler.handle(result);

        Ok(())
    }

    /// Renders a PDF from the message and emits it as processed output.
    ///
    fn process_pdf<'a>(
        &self,
        message: &Message<'a>,
        path: impl AsRef<path::Path>,
        dupe_id: impl Into<String>,
        context: &ProcessContext,
    ) -> anyhow::Result<()> {
        let mut writer = File::create(&path)?;
        let result = self.render_pdf(&message, &mut writer).map(|_| {
            ProcessOutput {
                path: path.as_ref().to_path_buf(),
                output_type: ProcessOutputType::Processed,
                mimetype: "application/pdf".to_string(),
                dupe_id: dupe_id.into(),
            }
        });
        context.result_handler.handle(result);

        Ok(())
    }

    /// Discovers any attachments in the message and emits them as embedded output.
    ///
    fn process_attachments<'a>(&self, message: &Message<'a>, context: &ProcessContext) -> anyhow::Result<()> {
        for part_id in &message.attachments {
            let part = message
                .part(*part_id)
                .ok_or(anyhow!("failed to get attachment part"))?;
            let content_type = part
                .content_type()
                .ok_or(anyhow!("failed to get attachment content type"))?;
            let mimetype = mimetype(content_type);
            let Workspace { original_path, dupe_id, .. } = Workspace::new(
                &part.contents(),
                &context.output_dir,
                &mimetype,
                &vec![]
            ).await?;

            context.result_handler.handle(Ok(ProcessOutput {
                path: original_path,
                output_type: ProcessOutputType::Embedded,
                mimetype,
                dupe_id,
            }));
        }

        Ok(())
    }
}

impl Process for Rfc822Processor {
    fn process(&self, content: &mut dyn AsyncRead, context: ProcessContext) {
        parse_message(content).await
            .and_then(|message| self.process(message, context))
            .unwrap_or_else(|err| context.result_handler.handle(Err(err)));
    }
}

fn parse_message<'a>(raw: impl AsyncRead) -> anyhow::Result<Message<'a>> {
    Message::parse(raw).ok_or(anyhow!("failed to parse message"))
}
