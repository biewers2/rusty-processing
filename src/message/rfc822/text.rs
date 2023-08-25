use std::fs::File;
use std::path;

use mail_parser::Message;

use crate::common::error::{ProcessError, ProcessResult};
use crate::message::rfc822::text_message_visitor::TextMessageVisitor;
use crate::message::rfc822::transformer::MessageTransformer;

pub fn extract(message: &Message, output_path: path::PathBuf) -> ProcessResult<()> {
  let transformer = MessageTransformer::new(Box::<TextMessageVisitor>::default());
  let mut file = File::create(output_path).map_err(ProcessError::Io)?;

  transformer.transform(message, &mut file)
}