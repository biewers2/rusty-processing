use std::{fs, path};
use std::borrow::Cow;
use std::fs::File;
use std::io::Write;

use crate::common::error::{ProcessError, ProcessResult};
use crate::common::output::OutputType;
use crate::dupe_id::identify_dupe::{IdentifyDupe, IdentifyDupeService};

pub struct Workspace {
  pub dupe_id: String,
  pub original_path: path::PathBuf,
  pub text_path: Option<path::PathBuf>,
  pub metadata_path: Option<path::PathBuf>,
  pub pdf_path: Option<path::PathBuf>,
}

pub struct WorkspaceOptions<'a, 'b> {
  pub dupe_identifier: &'static IdentifyDupeService,
  pub file_ext: String,
  pub output_dir: &'a path::PathBuf,
  pub types: &'b Vec<OutputType>,
}

pub fn create_from_raw(
  raw: &Cow<[u8]>,
  options: WorkspaceOptions,
) -> ProcessResult<Workspace> {
  let dupe_id = options.dupe_identifier.lock().unwrap().identify(&raw);

  let file_output_dir = options.output_dir.join(&dupe_id);
  fs::create_dir(&file_output_dir)
    .map_err(|err| ProcessError::from_io(err, "Failed to create output directory"))?;

  let original_path = file_output_dir.join(format!("original.{}", options.file_ext));
  File::create(&original_path)
    .and_then(|mut file| file.write_all(&raw))
    .map_err(|err| ProcessError::from_io(err, "Failed to write to original file path"))?;

  Ok(
    Workspace {
      dupe_id,
      original_path,

      text_path:
        options.types.contains(&OutputType::Text)
          .then(|| file_output_dir.join("extracted.txt")),

      metadata_path:
        options.types.contains(&OutputType::Metadata)
          .then(|| file_output_dir.join("metadata.json")),

      pdf_path:
        options.types.contains(&OutputType::Pdf)
          .then(|| file_output_dir.join("rendered.pdf")),
    }
  )
}
