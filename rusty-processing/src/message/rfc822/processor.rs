use std::borrow::Cow;
use std::{fs, path, thread};

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

    pub fn process(&self, message: Message) {
        if let Err(err) = self.process_with_result(message) {
            self.context.send_result(Err(err));
        }
    }

    fn process_with_result(&self, message: Message) -> anyhow::Result<()> {
        let wkspace = Workspace::new(&self.context, &message.raw_message)?;

        thread::scope(|s| {
            let text_path = wkspace.text_path;
            let dupe_id = wkspace.dupe_id.clone();
            s.spawn(|| {
                if let Some(path) = text_path {
                    self.context
                        .send_result(self.extract_text(&message, &path).map(|_| {
                            Output::Processed(OutputInfo {
                                path,
                                mimetype: "text/plain".to_string(),
                                dupe_id,
                            })
                        }));
                }
            });

            let metadata_path = wkspace.metadata_path;
            let dupe_id = wkspace.dupe_id.clone();
            s.spawn(|| {
                if let Some(path) = metadata_path {
                    self.context
                        .send_result(self.extract_metadata(&message, &path).map(|_| {
                            Output::Processed(OutputInfo {
                                path,
                                mimetype: "application/json".to_string(),
                                dupe_id,
                            })
                        }));
                }
            });

            let pdf_path = wkspace.pdf_path;
            let dupe_id = wkspace.dupe_id.clone();
            s.spawn(|| {
                if let Some(path) = pdf_path {
                    self.context
                        .send_result(self.render_pdf(&message, &path).map(|_| {
                            Output::Processed(OutputInfo {
                                path,
                                mimetype: "application/pdf".to_string(),
                                dupe_id,
                            })
                        }));
                }
            });

            if let Err(err) = self.process_attachments(&message) {
                self.context.send_result(Err(err));
            }
        });

        Ok(())
    }

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
