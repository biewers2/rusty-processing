use std::path;
use std::sync::Mutex;
use lazy_static::lazy_static;

use crate::common::error::ProcessResult;
use crate::common::output::OutputType;
use crate::processing::process::{default_types, process_mime};

lazy_static! {
  static ref PROCESSOR: Mutex<Processor> = Mutex::new(Processor {});
}

pub fn processor() -> &'static Mutex<Processor> {
  &PROCESSOR
}

pub struct Processor {}

impl Processor {
  pub fn process_file(
    &self,
    source_file: &path::PathBuf,
    output_dir: &path::PathBuf,
    mimetype: &String,
    types: Option<&Vec<OutputType>>
  ) -> ProcessResult<()> {
    process_mime(mimetype, |processor| {
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
    process_mime(mimetype, |processor| {
      processor.handle_raw(
        raw,
        output_dir,
        types.unwrap_or(&default_types())
      )
    })
  }
}