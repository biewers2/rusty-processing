use std::borrow::Cow;
use std::fs::File;
use std::io::Write;
use std::{fs, path};

use crate::common::error::{ProcessError, ProcessResult};
use crate::common::output_type::OutputType;
use crate::dupe_id::identify_dupe::IdentifyDupeService;
use crate::processing::context::Context;

pub struct Workspace {
    pub dupe_id: String,
    pub original_path: path::PathBuf,
    pub text_path: Option<path::PathBuf>,
    pub metadata_path: Option<path::PathBuf>,
    pub pdf_path: Option<path::PathBuf>,
}

pub struct WorkspaceOptions<'a> {
    pub dupe_identifier: &'static IdentifyDupeService,
    pub file_ext: String,
    pub context: &'a Context,
}

pub fn create_from_raw(raw: &Cow<[u8]>, options: WorkspaceOptions) -> ProcessResult<Workspace> {
    let dupe_id = options.dupe_identifier.identify(&raw);

    let file_output_dir = options.context.output_dir.join(&dupe_id);
    fs::create_dir(&file_output_dir)
        .map_err(|_| ProcessError::duplicate(&options.context, &dupe_id))?;

    let original_path = file_output_dir.join(format!("original.{}", options.file_ext));
    File::create(&original_path)
        .and_then(|mut file| file.write_all(&raw))
        .map_err(|err| {
            ProcessError::from_io(
                &options.context,
                err,
                "Failed to write to original file path",
            )
        })?;

    Ok(Workspace {
        dupe_id,
        original_path,

        text_path: options
            .context
            .should_process_type(&OutputType::Text)
            .then(|| file_output_dir.join("extracted.txt")),

        metadata_path: options
            .context
            .should_process_type(&OutputType::Metadata)
            .then(|| file_output_dir.join("metadata.json")),

        pdf_path: options
            .context
            .should_process_type(&OutputType::Pdf)
            .then(|| file_output_dir.join("rendered.pdf")),
    })
}
