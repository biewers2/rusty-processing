use std::path;
use crate::application::mbox::processor::mbox_processor;

use crate::common::error::{ProcessError, ProcessResult};
use crate::common::output::OutputType;
use crate::message::rfc822::processor::rfc822_processor;

const TYPES: [OutputType; 3] = [
  OutputType::Metadata,
  OutputType::Text,
  OutputType::Pdf
];

pub(super) fn default_types() -> Vec<OutputType> {
  return TYPES.to_vec()
}

pub type ProcessService = Box<dyn Process>;

pub trait Process: Send + Sync {
  fn handle_file(
    &self,
    source_file: &path::PathBuf,
    output_dir: &path::PathBuf,
    types: &Vec<OutputType>,
  ) -> ProcessResult<()>;

  fn handle_raw(
    &self,
    raw: &[u8],
    output_dir: &path::PathBuf,
    types: &Vec<OutputType>,
  ) -> ProcessResult<()>;
}
