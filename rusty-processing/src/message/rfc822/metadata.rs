use std::io::Write;

use json::object;
use mail_parser::{Message, MimeHeaders};

use crate::message::rfc822::processor::Rfc822Processor;

impl Rfc822Processor {
    pub fn extract_metadata<W>(
        &self,
        message: &Message,
        writer: &mut W
    ) -> anyhow::Result<()>
        where W: Write,
    {
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
        writer.write_all(metadata_json.as_bytes())
            .and(writer.write_all(b"\n"))?;

        Ok(())
    }

    fn format_attachment_names(&self, message: &Message) -> Option<String> {
        let formatted_atts = message
            .attachments()
            .filter_map(|att| att.attachment_name())
            .collect::<Vec<&str>>()
            .join(", ");
        (!formatted_atts.is_empty()).then_some(formatted_atts)
    }
}
