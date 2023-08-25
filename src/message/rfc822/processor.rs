use std::{fs, path, thread};
use std::sync::mpsc;

use lazy_static::lazy_static;
use mail_parser::Message;

use crate::common::error::{ProcessError, ProcessResult};
use crate::common::output::OutputType;
use crate::common::workspace;
use crate::common::workspace::{Workspace, WorkspaceOptions};
use crate::dupe_id::message_dupe_identifier::message_dupe_identifier;
use crate::message::rfc822::{metadata, pdf, text};
use crate::processing::process::{Process, ProcessService};

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
    output_dir: &path::PathBuf,
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

    let text_tx = tx.clone();
    let metadata_tx = tx.clone();
    let pdf_tx = tx.clone();

    thread::scope(|_| {
      text_tx.send(
        if let Some(path) = text_path {
          text::extract(&message, path)
        } else {
          Ok(())
        }
      ).unwrap();
    });
    thread::scope(|_| {
      metadata_tx.send(
        if let Some(path) = metadata_path {
          metadata::extract(&message, path)
        } else {
          Ok(())
        }
      ).unwrap();
    });
    thread::scope(|_| {
      pdf_tx.send(
        if let Some(path) = pdf_path {
          pdf::render(&message, path)
        } else {
          Ok(())
        }
      ).unwrap();
    });

    for _ in 0..3 {
      rx.recv().unwrap()?;
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
    source_file: &path::PathBuf,
    output_dir: &path::PathBuf,
    types: &Vec<OutputType>,
  ) -> ProcessResult<()> {
    let raw = fs::read_to_string(source_file).map_err(ProcessError::Io)?;
    self.parse_message(raw.as_bytes())
      .and_then(|message| self.process(message, output_dir, types))
  }

  fn handle_raw(
    &self,
    raw: &[u8],
    output_dir: &path::PathBuf,
    types: &Vec<OutputType>,
  ) -> ProcessResult<()> {
    self.parse_message(raw)
      .and_then(|message| self.process(message, output_dir, types))
  }
}
