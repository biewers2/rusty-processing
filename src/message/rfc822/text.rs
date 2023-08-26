use std::fs::File;
use std::path;

use mail_parser::Message;

use crate::common::error::{ProcessError, ProcessResult};
use crate::message::rfc822::processor::Rfc822Processor;
use crate::message::rfc822::text_message_visitor::TextMessageVisitor;
use crate::message::rfc822::transformer::MessageTransformer;

impl Rfc822Processor {
    pub fn extract_text(&self, message: &Message, output_path: path::PathBuf) -> ProcessResult<()> {
        let transformer = MessageTransformer::new(Box::<TextMessageVisitor>::default());
        let mut file =
            File::create(output_path).map_err(|err| ProcessError::io(&self.context, err))?;

        transformer
            .transform(message, &mut file)
            .map_err(|err| ProcessError::from_io(&self.context, err, "Failed to transform message"))
    }
}
