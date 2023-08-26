use std::{fs, path, thread};

use mail_parser::{Message, MimeHeaders};

use crate::common::error::{ProcessError, ProcessResult};
use crate::common::output_type::OutputType;
use crate::common::workspace;
use crate::common::workspace::{Workspace, WorkspaceOptions};
use crate::dupe_id::message_dupe_identifier::message_dupe_identifier;
use crate::processing::context::Context;
use crate::processing::process::Process;
use crate::processing::processor::processor;

const FILE_EXT: &str = "eml";

pub struct Rfc822Processor {
    pub(super) context: Context,
}

impl Rfc822Processor {
    pub fn new(context: Context) -> Self {
        Self { context }
    }

    pub fn process(&self, message: Message) {
        let options = WorkspaceOptions {
            dupe_identifier: message_dupe_identifier(),
            file_ext: FILE_EXT.to_string(),
            context: &self.context,
        };

        let Workspace {
            dupe_id: _digest,
            original_path: _original_path,
            text_path,
            metadata_path,
            pdf_path,
        } = match workspace::create_from_raw(&message.raw_message, options) {
            Ok(workspace) => workspace,
            Err(err) => {
                self.context.send_result(Err(err));
                return;
            }
        };

        let (output_dir, types) = (self.context.output_dir.clone(), self.context.types.clone());

        thread::scope(|s| {
            s.spawn(|| {
                self.context
                    .send_result(self.process_text(&message, text_path))
            });
            s.spawn(|| {
                self.context
                    .send_result(self.process_metadata(&message, metadata_path))
            });
            s.spawn(|| {
                self.context
                    .send_result(self.process_pdf(&message, pdf_path))
            });
            s.spawn(|| {
                self.context
                    .send_result(self.process_attachments(&message, output_dir, types))
            });
        });
    }

    fn process_text(&self, message: &Message, path: Option<path::PathBuf>) -> ProcessResult<()> {
        match path {
            Some(path) => self.extract_text(message, path),
            None => Ok(()),
        }
    }

    fn process_metadata(
        &self,
        message: &Message,
        path: Option<path::PathBuf>,
    ) -> ProcessResult<()> {
        match path {
            Some(path) => self.extract_metadata(message, path),
            None => Ok(()),
        }
    }

    fn process_pdf(&self, message: &Message, path: Option<path::PathBuf>) -> ProcessResult<()> {
        match path {
            Some(path) => self.render_pdf(message, path),
            None => Ok(()),
        }
    }

    fn process_attachments(
        &self,
        message: &Message,
        output_dir: path::PathBuf,
        types: Option<Vec<OutputType>>,
    ) -> ProcessResult<()> {
        for part_id in &message.attachments {
            let part = message.part(*part_id).ok_or(ProcessError::unexpected(
                &self.context,
                "attachment not found",
            ))?;

            let content_type = part.content_type().ok_or(ProcessError::unexpected(
                &self.context,
                "attachment missing content type",
            ))?;

            let mimetype = match (content_type.ctype(), content_type.subtype()) {
                (ctype, Some(subtype)) => format!("{}/{}", ctype, subtype),
                (ctype, None) => ctype.to_string(),
            };

            processor().process_raw(
                part.contents(),
                output_dir.clone(),
                mimetype.clone(),
                types.clone(),
                |result| self.context.send_result(result),
            );
        }

        Ok(())
    }

    fn parse_message<'a>(&'a self, raw: &'a [u8]) -> Result<Message, ProcessError> {
        Message::parse(raw).ok_or(ProcessError::unexpected(
            &self.context,
            "message failed to parse",
        ))
    }
}

impl Process for Rfc822Processor {
    fn handle_file(&self, source_file: &path::PathBuf) {
        match fs::read_to_string(source_file) {
            Ok(raw) => self
                .parse_message(raw.as_bytes())
                .map(|message| self.process(message))
                .unwrap_or_else(|err| self.context.send_result(Err(err))),
            Err(err) => self
                .context
                .send_result(Err(ProcessError::io(&self.context, err))),
        }
    }

    fn handle_raw(&self, raw: &[u8]) {
        self.parse_message(raw)
            .map(|message| self.process(message))
            .unwrap_or_else(|err| self.context.send_result(Err(err)));
    }
}
