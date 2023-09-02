use std::io::{self};
use std::path;

use crate::common::mime_extension_map::map_to_file_ext;
use crate::common::output_type::ProcessType;
use crate::common::util;
use crate::dupe_id::identify_dupe::identifier;
use crate::processing::context::Context;

pub struct Workspace {
    pub dupe_id: String,
    pub entry_dir: path::PathBuf,
    pub original_path: path::PathBuf,
    pub text_path: Option<path::PathBuf>,
    pub metadata_path: Option<path::PathBuf>,
    pub pdf_path: Option<path::PathBuf>,
}

impl Workspace {
    pub fn new(context: &Context, content: &[u8]) -> anyhow::Result<Workspace> {
        let dupe_id = identifier(&context.mimetype).identify(content);
        let dir = context.output_dir.join(&dupe_id);

        let original_path = dir.join(format!("original.{}", map_to_file_ext(&context.mimetype)));
        util::write_file(&original_path, content)?;

        let text_path = context
            .should_process_type(&ProcessType::Text)
            .then(|| dir.join("extracted.txt"))
            .and_then(|path| (!path.exists()).then(|| path));
        let metadata_path = context
            .should_process_type(&ProcessType::Metadata)
            .then(|| dir.join("metadata.json"))
            .and_then(|path| (!path.exists()).then(|| path));
        let pdf_path = context
            .should_process_type(&ProcessType::Pdf)
            .then(|| dir.join("rendered.pdf"))
            .and_then(|path| (!path.exists()).then(|| path));

        Ok(Workspace {
            dupe_id,
            entry_dir: dir,
            original_path,
            text_path,
            metadata_path,
            pdf_path,
        })
    }
}
