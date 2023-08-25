use std::path;
use lazy_static::lazy_static;
use crate::application::mbox::processor::mbox_processor;

use crate::common::error::{ProcessError, ProcessResult};
use crate::common::output::OutputType;
use crate::message::rfc822::processor::rfc822_processor;
use crate::processing::process::{default_types, Process};

lazy_static! {
  static ref PROCESSOR: Processor = Processor::default();
}

pub fn processor() -> &'static Processor {
  &PROCESSOR
}

#[derive(Default)]
pub struct Processor {}

impl Processor {
  pub fn process_file(
    &self,
    source_file: &path::PathBuf,
    output_dir: &path::PathBuf,
    mimetype: &String,
    types: Option<&Vec<OutputType>>
  ) -> ProcessResult<()> {
    self.process_mime(mimetype, |processor| {
      processor.handle_file(
        source_file,
        output_dir,
        types.unwrap_or(&default_types())
      )
    })
  }

  pub fn process_raw(
    &self,
    raw: &[u8],
    output_dir: &path::PathBuf,
    mimetype: &String,
    types: Option<&Vec<OutputType>>
  ) -> ProcessResult<()> {
    self.process_mime(mimetype, |processor| {
      processor.handle_raw(
        raw,
        output_dir,
        types.unwrap_or(&default_types())
      )
    })
  }

  pub(crate) fn process_mime<T, F>(&self, mimetype: &String, block: F) -> ProcessResult<T>
    where F: Fn(&Box<dyn Process>) -> ProcessResult<T>
  {
    match mimetype.as_str() {
      "application/mbox" => block(&mbox_processor()),
      "message/rfc822" => block(&rfc822_processor()),
      _ => {
        println!("no processor for mimetype {}", mimetype);
        Err(ProcessError::from("no processor for mimetype"))
      }
    }
  }
}