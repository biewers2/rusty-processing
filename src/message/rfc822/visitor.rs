use std::borrow::Cow;

use mail_parser::{Addr, ContentType, DateTime, Group};

pub trait MessageVisitor {
  fn on_header_prefix(&self) -> Option<String> { None }
  fn on_header_suffix(&self) -> Option<String> { None }
  fn on_head_body_separator(&self) -> Option<String> { None }
  fn on_part_prefix(&self) -> Option<String> { None }
  fn on_part_suffix(&self) -> Option<String> { None }

  // Header visitors
  fn on_header_address(&self, name: &str, address: &Addr) -> Option<String>;
  fn on_header_address_list(&self, name: &str, address_list: &Vec<Addr>) -> Option<String>;
  fn on_header_group(&self, name: &str, group: &Group) -> Option<String>;
  fn on_header_group_list(&self, name: &str, group_list: &Vec<Group>) -> Option<String>;
  fn on_header_text(&self, name: &str, text: &Cow<str>) -> Option<String>;
  fn on_header_text_list(&self, name: &str, text_list: &Vec<Cow<str>>) -> Option<String>;
  fn on_header_date_time(&self, name: &str, date_time: &DateTime) -> Option<String>;
  fn on_header_content_type(&self, content_type: &ContentType) -> Option<String>;

  // Body part visitors
  fn on_part_text(&self, value: &Cow<str>) -> String;
  fn on_part_html(&self, value: &Cow<str>) -> String;
  fn on_part_binary(&self, value: &Cow<[u8]>) -> Vec<u8>;
  fn on_part_inline_binary(&self, value: &Cow<[u8]>) -> Vec<u8>;
}