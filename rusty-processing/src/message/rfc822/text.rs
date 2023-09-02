use std::{io, path};
use std::fs::File;

use mail_parser::Message;

use crate::message::rfc822::processor::Rfc822Processor;
use crate::message::rfc822::text_message_visitor::TextMessageVisitor;
use crate::message::rfc822::transformer::MessageTransformer;

impl Rfc822Processor {
    pub fn extract_text(&self, message: &Message, output_path: &path::PathBuf) -> anyhow::Result<()> {
        let transformer = MessageTransformer::new(Box::<TextMessageVisitor>::default());
        let mut file = File::create(output_path)?;
        transformer.transform(message, &mut file)?;
        Ok(())
    }
}
