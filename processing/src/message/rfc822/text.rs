use std::io::Write;

use mail_parser::Message;

use crate::message::rfc822::processor::Rfc822Processor;
use crate::message::rfc822::text_message_visitor::TextMessageVisitor;
use crate::message::rfc822::transformer::MessageTransformer;

impl Rfc822Processor {
    pub fn extract_text<W>(
        &self,
        message: &Message,
        writer: &mut W,
    ) -> anyhow::Result<()>
        where W: Write,
    {
        let transformer = MessageTransformer::new(Box::<TextMessageVisitor>::default());
        transformer.transform(message, writer)?;
        Ok(())
    }
}
