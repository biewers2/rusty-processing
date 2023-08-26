use json::object;
use std::fs::File;
use std::io::Write;
use std::path;

use mail_parser::{Message, MimeHeaders};

use crate::common::error::{ProcessError, ProcessResult};
use crate::message::rfc822::processor::Rfc822Processor;

impl Rfc822Processor {
    pub fn extract_metadata(
        &self,
        message: &Message,
        output_path: path::PathBuf,
    ) -> ProcessResult<()> {
        let mut metadata = object! {};

        for (key, value) in message.headers_raw() {
            let value = value.trim();
            (!value.is_empty()).then(|| metadata[key] = value.into());
        }

        metadata["File-Extension"] = "eml".into();
        metadata["File-Size"] = message.raw_message().len().into();

        metadata["Has-Attachments"] = (message.attachment_count() > 0).into();
        metadata["Attachment-Count"] = message.attachment_count().into();
        self.format_attachment_names(message)
            .map(|atts| metadata["Attachment-Names"] = atts.into());

        let metadata_json = json::stringify_pretty(metadata, 2);
        File::create(output_path)
            .and_then(|mut file| {
                file.write_all(metadata_json.as_bytes())
                    .and(file.write_all(b"\n"))
            })
            .map_err(|err| {
                ProcessError::from_io(&self.context, err, "Failed to write metadata file")
            })
    }

    fn format_attachment_names(&self, message: &Message) -> Option<String> {
        let formatted_atts = message
            .attachments()
            .map(|att| att.attachment_name())
            .filter(|att| att.is_some())
            .map(|att| att.unwrap())
            .collect::<Vec<&str>>()
            .join(", ");
        (!formatted_atts.is_empty()).then(|| formatted_atts)
    }
}
