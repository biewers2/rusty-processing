use std::borrow::Cow;

use mail_parser::{Addr, ContentType, DateTime, Group};
use crate::message::rfc822::message_formatter::MessageFormatter;

use crate::message::rfc822::message_visitor::MessageVisitor;

#[derive(Default)]
pub struct HtmlMessageVisitor {
  formatter: MessageFormatter,
}

impl MessageVisitor for HtmlMessageVisitor {
  fn on_header_prefix(&self) -> Option<String> {
    Some("<div>".to_string())
  }

  fn on_header_suffix(&self) -> Option<String> {
    Some("</div>".to_string())
  }

  fn on_head_body_separator(&self) -> Option<String> {
    Some("<br>\n".to_string())
  }

  fn on_part_prefix(&self) -> Option<String> {
    Some("<div>".to_string())
  }

  fn on_part_suffix(&self) -> Option<String> {
    Some("</div>".to_string())
  }

  fn on_header_address(&self, header_name: &str, address: &Addr) -> Option<String> {
    self.formatter.format_address(address)
      .map(|value| format!("<b>{}</b>: {}", header_name, value))
  }

  fn on_header_address_list(&self, name: &str, address_list: &Vec<Addr>) -> Option<String> {
    self.formatter.format_address_list(address_list)
      .map(|addrs| format!("<b>{}</b>: {}", name, addrs))
  }

  fn on_header_group(&self, header_name: &str, group: &Group) -> Option<String> {
    self.formatter.format_group(group)
      .map(|value| format!("<b>{}</b>: {}", header_name, value))
  }

  fn on_header_group_list(&self, name: &str, group_list: &Vec<Group>) -> Option<String> {
    self.formatter.format_group_list(group_list)
      .map(|groups| format!("<b>{}</b>: {}", name, groups))
  }

  fn on_header_text(&self, name: &str, text: &Cow<str>) -> Option<String> {
    Some(format!("<b>{}</b>: {}", name, text))
  }

  fn on_header_text_list(&self, name: &str, text_list: &Vec<Cow<str>>) -> Option<String> {
    self.formatter.format_text_list(text_list)
      .map(|texts| format!("<b>{}</b>: {}", name, texts))
  }

  fn on_header_date_time(&self, name: &str, date_time: &DateTime) -> Option<String> {
    Some(format!("<b>{}</b>: {}", name, date_time))
  }

  fn on_header_content_type(&self, _: &ContentType) -> Option<String> {
    None
  }
}
