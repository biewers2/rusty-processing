use std::borrow::Cow;
use mail_parser::{Addr, ContentType, DateTime, Group};
use crate::message::rfc822::message_formatter::MessageFormatter;
use crate::message::rfc822::message_visitor::MessageVisitor;

#[derive(Default)]
pub struct TextMessageVisitor {
  formatter: MessageFormatter,
}

impl MessageVisitor for TextMessageVisitor {
  fn on_head_body_separator(&self) -> Option<String> {
    Some("\n".to_string())
  }

  fn on_header_address(&self, header_name: &str, address: &Addr) -> Option<String> {
    self.formatter.format_address(address)
      .map(|value| format!("{}: {}", header_name, value))
  }

  fn on_header_address_list(&self, name: &str, address_list: &Vec<Addr>) -> Option<String> {
    self.formatter.format_address_list(address_list)
      .map(|addrs| format!("{}: {}", name, addrs))
  }

  fn on_header_group(&self, header_name: &str, group: &Group) -> Option<String> {
    self.formatter.format_group(group)
      .map(|value| format!("{}: {}", header_name, value))
  }

  fn on_header_group_list(&self, name: &str, group_list: &Vec<Group>) -> Option<String> {
    self.formatter.format_group_list(group_list)
      .map(|groups| format!("{}: {}", name, groups))
  }

  fn on_header_text(&self, name: &str, text: &Cow<str>) -> Option<String> {
    Some(format!("{}: {}", name, text))
  }

  fn on_header_text_list(&self, name: &str, text_list: &Vec<Cow<str>>) -> Option<String> {
    self.formatter.format_text_list(text_list)
      .map(|texts| format!("{}: {}", name, texts))
  }

  fn on_header_date_time(&self, name: &str, date_time: &DateTime) -> Option<String> {
    Some(format!("{}: {}", name, date_time))
  }

  fn on_header_content_type(&self, _: &ContentType) -> Option<String> {
    None
  }

  fn on_part_html(&self, value: &Cow<str>) -> String {
    html2text::from_read(value.as_bytes(), 120)
  }
}