use std::io::Write;

use mail_parser::{HeaderValue, Message, MessagePart, PartType};

use crate::common::error::{ProcessError, ProcessResult};
use crate::message::rfc822::message_visitor::MessageVisitor;

const HEADERS: [&str; 6] = ["Date", "From", "To", "CC", "BCC", "Subject"];

pub struct MessageTransformer {
  visitor: Box<dyn MessageVisitor>,
}

impl MessageTransformer {
  pub fn new(visitor: Box<dyn MessageVisitor>) -> Self {
    Self { visitor }
  }

  pub fn transform<W>(&self, message: &Message, writer: &mut W) -> ProcessResult<()>
    where W: Write
  {
    for header in HEADERS {
      if let Some(header_value) = self.transform_header(message, header) {
        self.write_if_some(writer, self.visitor.on_header_prefix())?;

        let value = format!("{}", header_value);
        writer.write_all(value.as_bytes())
          .map_err(ProcessError::Io)?;

        self.write_if_some(writer, self.visitor.on_header_suffix())?;
        writer.write_all(b"\n")
          .map_err(ProcessError::Io)?;
      }
    }

    self.write_if_some(writer, self.visitor.on_head_body_separator())?;

    let bodies =
      if message.html_body_count() > 0 {
        message.html_bodies()
      } else {
        message.text_bodies()
      };
    for part in bodies {
      self.write_if_some(writer, self.visitor.on_part_prefix())?;
      self.transform_part(message, writer, part)?;
      self.write_if_some(writer, self.visitor.on_part_suffix())?;
    }

    Ok(())
  }

  fn transform_header(&self, message: &Message, header_name: &str) -> Option<String> {
    let header_value = message.header(header_name).unwrap_or(&HeaderValue::Empty);
    match header_value {
      HeaderValue::Address(addr) =>
        self.visitor.on_header_address(header_name, addr),

      HeaderValue::AddressList(addr_list) =>
        self.visitor.on_header_address_list(header_name, addr_list),

      HeaderValue::Group(group) =>
        self.visitor.on_header_group(header_name, group),

      HeaderValue::GroupList(group_list) =>
        self.visitor.on_header_group_list(header_name, group_list),

      HeaderValue::Text(text) =>
        self.visitor.on_header_text(header_name, text),

      HeaderValue::TextList(text_list) =>
        self.visitor.on_header_text_list(header_name, text_list),

      HeaderValue::DateTime(date_time) =>
        self.visitor.on_header_date_time(header_name, date_time),

      HeaderValue::ContentType(_) | HeaderValue::Empty => None,
    }
  }

  fn transform_part<W>(&self, message: &Message, writer: &mut W, part: &MessagePart) -> ProcessResult<()>
    where W: Write
  {
    match &part.body {
      PartType::Text(text) => {
        let text = self.visitor.on_part_text(text);
        writer.write_all(text.as_bytes())
          .map_err(ProcessError::Io)?;
      }

      PartType::Html(html) => {
        let html = self.visitor.on_part_html(html);
        writer.write_all(html.as_bytes())
          .map_err(ProcessError::Io)?;
      },

      PartType::Binary(binary) => {
        let binary = self.visitor.on_part_binary(binary);
        writer.write_all(binary.as_ref())
          .map_err(ProcessError::Io)?
      },

      PartType::InlineBinary(inline_binary) => {
        let inline_binary = self.visitor.on_part_inline_binary(inline_binary);
        writer.write_all(inline_binary.as_ref())
          .map_err(ProcessError::Io)?
      },

      PartType::Message(message) =>
        self.transform(message, writer)?,

      PartType::Multipart(part_ids) => {
        for part_id in part_ids {
          if let Some(p) = message.part(*part_id) {
            self.transform_part(message, writer, p)?;
          }
        }
      },
    };

    Ok(())
  }

  fn write_if_some<W>(&self, writer: &mut W, value: Option<String>) -> ProcessResult<()>
    where W: Write
  {
    if let Some(value) = value {
      writer.write_all(value.as_bytes()).map_err(ProcessError::Io)?;
    }
    Ok(())
  }
}
