use std::{fs, path, thread};
use std::path::PathBuf;
use std::sync::mpsc;

use lazy_static::lazy_static;
use mail_parser::{Message, MimeHeaders};

use crate::common::error::{ProcessError, ProcessResult};
use crate::common::output::OutputType;
use crate::common::workspace;
use crate::common::workspace::{Workspace, WorkspaceOptions};
use crate::dupe_id::message_dupe_identifier::message_dupe_identifier;
use crate::message::rfc822::{metadata, pdf, text};
use crate::processing::process::{Process, ProcessService};
use crate::processing::processor::processor;

const FILE_EXT: &str = "eml";

lazy_static! {
  static ref RFC822_PROCESSOR: ProcessService = Box::<Rfc822Processor>::default();
}

pub fn rfc822_processor() -> &'static ProcessService {
  &RFC822_PROCESSOR
}

#[derive(Default)]
pub struct Rfc822Processor {}

impl Rfc822Processor {
  pub fn process(
    &self,
    message: Message,
    output_dir: &PathBuf,
    types: &Vec<OutputType>,
  ) -> ProcessResult<()> {
    let options = WorkspaceOptions {
      dupe_identifier: message_dupe_identifier(),
      file_ext: FILE_EXT.to_string(),
      output_dir,
      types,
    };

    let Workspace {
      dupe_id: _digest,
      original_path: _original_path,
      text_path,
      metadata_path,
      pdf_path,
    } = workspace::create_from_raw(
      &message.raw_message,
      options
    )?;

    let (tx, rx) = mpsc::channel();
    let (
      text_tx,
      metadata_tx,
      pdf_tx,
      att_tx
    ) = (tx.clone(), tx.clone(), tx.clone(), tx.clone());
    let (output_dir, types) = (output_dir.clone(), types.clone());

    let thread_count = vec![
      thread::scope(|_| text_tx.send(self.process_text(&message, text_path)).unwrap()),
      thread::scope(|_| metadata_tx.send(self.process_metadata(&message, metadata_path)).unwrap()),
      thread::scope(|_| pdf_tx.send(self.process_pdf(&message, pdf_path)).unwrap()),
      thread::scope(|_| att_tx.send(self.process_attachments(&message, &output_dir, &types)).unwrap()),
    ].len();

    for _ in 0..thread_count {
      rx.recv().unwrap()?;
    }

    Ok(())
  }

  fn process_text(&self, message: &Message, path: Option<PathBuf>) -> ProcessResult<()> {
    match path {
      Some(path) => text::extract(message, path),
      None => Ok(())
    }
  }

  fn process_metadata(&self, message: &Message, path: Option<PathBuf>) -> ProcessResult<()> {
    match path {
      Some(path) => metadata::extract(message, path),
      None => Ok(())
    }
  }

  fn process_pdf(&self, message: &Message, path: Option<PathBuf>) -> ProcessResult<()> {
    match path {
      Some(path) => pdf::render(message, path),
      None => Ok(())
    }
  }

  fn process_attachments(&self, message: &Message, output_dir: &PathBuf, types: &Vec<OutputType>) -> ProcessResult<()> {
    for part_id in &message.attachments {
      let part =
        message.part(*part_id)
          .ok_or(ProcessError::from("attachment not found"))?;

      let content_type =
        part.content_type()
          .ok_or(ProcessError::from("attachment missing content type"))?;

      let mimetype = match (content_type.ctype(), content_type.subtype()) {
        (ctype, Some(subtype)) => format!("{}/{}", ctype, subtype),
        (ctype, None) => ctype.to_string()
      };

      processor().process_raw(part.contents(), output_dir, &mimetype, Some(types))?;
    }

    Ok(())
  }

  fn parse_message<'a>(&'a self, raw: &'a [u8]) -> Result<Message, ProcessError> {
    Message::parse(raw)
      .ok_or(ProcessError::from("message failed to parse"))
  }
}

impl Process for Rfc822Processor {
  fn handle_file(
    &self,
    source_file: &PathBuf,
    output_dir: &PathBuf,
    types: &Vec<OutputType>,
  ) -> ProcessResult<()> {
    let raw = fs::read_to_string(source_file).map_err(ProcessError::Io)?;
    self.parse_message(raw.as_bytes())
      .and_then(|message| self.process(message, output_dir, types))
  }

  fn handle_raw(
    &self,
    raw: &[u8],
    output_dir: &PathBuf,
    types: &Vec<OutputType>,
  ) -> ProcessResult<()> {
    self.parse_message(raw)
      .and_then(|message| self.process(message, output_dir, types))
  }
}
