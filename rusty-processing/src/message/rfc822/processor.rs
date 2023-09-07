use std::{fs, path, thread};
use std::borrow::Cow;

use anyhow::anyhow;
use mail_parser::{Message, MimeHeaders};

use crate::common::util::mimetype;
use crate::common::workspace::Workspace;
use crate::processing::context::Context;
use crate::processing::output::{Output, OutputInfo};
use crate::processing::process::Process;

pub struct Rfc822Processor {
    pub(super) context: Context,
}

impl Rfc822Processor {
    pub fn new(context: Context) -> Self {
        Self { context }
    }

    /// Processes a message by extracting text and metadata, rendering a PDF, and then finding any embedded attachments.
    ///
    pub fn process(&self, message: Message) {
        let wkspace = match Workspace::new(&self.context, &message.raw_message) {
            Ok(ws) => ws,
            Err(e) => {
                self.context.send_result(Err(e));
                return;
            }
        };

        thread::scope(|s| {
            s.spawn(|| if let Err(e) = self.process_text(&message, &wkspace) {
                self.context.send_result(Err(e)); 
            });
            s.spawn(|| if let Err(e) = self.process_metadata(&message, &wkspace) {
                self.context.send_result(Err(e));
            });
            s.spawn(|| if let Err(e) = self.process_pdf(&message, &wkspace) {
                self.context.send_result(Err(e));
            });
            if let Err(e) = self.process_attachments(&message) {
                self.context.send_result(Err(e));
            }
        });
    }

    /// Extracts the text from the message and emits it as processed output.
    ///
    fn process_text(&self, message: &Message, wkspace: &Workspace) -> anyhow::Result<()> {
        if let (Some(path), Some(mut writer)) = (&wkspace.text_path, wkspace.text_writer()?) {
            let output = self.extract_text(&message, &mut writer).map(|_| {
                Output::Processed(OutputInfo {
                    path: path.to_owned(),
                    mimetype: "text/plain".to_string(),
                    dupe_id: wkspace.dupe_id.to_owned(),
                })
            });
            self.context.send_result(output);
        }
        Ok(())
    }

    /// Extracts the metadata from the message and emits it as processed output.
    ///
    fn process_metadata(&self, message: &Message, wkspace: &Workspace) -> anyhow::Result<()> {
        if let (Some(path), Some(mut writer)) = (&wkspace.metadata_path, wkspace.metadata_writer()?) {
            let output = self.extract_metadata(&message, &mut writer).map(|_| {
                Output::Processed(OutputInfo {
                    path: path.to_owned(),
                    mimetype: "application/json".to_string(),
                    dupe_id: wkspace.dupe_id.to_owned(),
                })
            });
            self.context.send_result(output);
        }
        Ok(())
    }

    /// Renders a PDF from the message and emits it as processed output.
    ///
    fn process_pdf(&self, message: &Message, wkspace: &Workspace) -> anyhow::Result<()> {
        if let (Some(path), Some(mut writer)) = (&wkspace.pdf_path, wkspace.pdf_writer()?) {
            let output = self.render_pdf(&message, &mut writer).map(|_| {
                Output::Processed(OutputInfo {
                    path: path.to_owned(),
                    mimetype: "application/pdf".to_string(),
                    dupe_id: wkspace.dupe_id.to_owned(),
                })
            });
            self.context.send_result(output);
        }
        Ok(())
    }

    /// Discovers any attachments in the message and emits them as embedded output.
    ///
    fn process_attachments(&self, message: &Message) -> anyhow::Result<()> {
        for part_id in &message.attachments {
            let part = message
                .part(*part_id)
                .ok_or(anyhow!("failed to get attachment part"))?;
            let content_type = part
                .content_type()
                .ok_or(anyhow!("failed to get attachment content type"))?;
            let mimetype = mimetype(content_type);
            let wkspace = Workspace::new(&self.context.with_mimetype(&mimetype), &part.contents())?;

            self.context.send_result(Ok(Output::Embedded(OutputInfo {
                path: wkspace.original_path,
                mimetype,
                dupe_id: wkspace.dupe_id,
            })));
        }
        Ok(())
    }
}

impl Process for Rfc822Processor {
    fn handle_file(&self, source_file: &path::PathBuf) {
        if let Ok(content) = fs::read_to_string(source_file) {
            parse_message(content.as_bytes())
                .map(|message| self.process(message))
                .unwrap_or_else(|err| self.context.send_result(Err(err)));
        } else {
            self.context
                .send_result(Err(anyhow!("failed to read file: {:?}", source_file)));
        }
    }

    fn handle_raw(&self, raw: Cow<[u8]>) {
        parse_message(&raw)
            .map(|message| self.process(message))
            .unwrap_or_else(|err| self.context.send_result(Err(err)));
    }
}

fn parse_message(raw: &[u8]) -> anyhow::Result<Message> {
    Message::parse(raw).ok_or(anyhow!("failed to parse message"))
}
