use std::borrow::Cow;
use std::io::Read;
use mail_parser::{Addr, ContentType, DateTime, Group};
use crate::message::rfc822::visitor::MessageVisitor;

#[derive(Default)]
pub struct HtmlMessageVisitor {}

impl HtmlMessageVisitor {
  fn format_address(&self, name: &Option<String>, address: &Option<String>) -> Option<String> {
    match (name, address) {
      (Some(name), Some(address)) => Some(format!("{} <{}>", name, address)),
      (Some(name), None) => Some(name.to_string()),
      (None, Some(address)) => Some(format!("<{}>", address)),
      (None, None) => None,
    }
  }

  fn extract_address_value(&self, address: &Addr) -> Option<String> {
    let name = address.name.as_ref().map(|s| s.to_string());
    let address = address.address.as_ref().map(|s| s.to_string());
    self.format_address(&name, &address)
  }

  fn extract_address_list_value(&self, address_list: &Vec<Addr>) -> Option<String> {
    (!address_list.is_empty())
      .then(||
        address_list.iter()
          .map(|addr| self.extract_address_value(&addr))
          .filter(|addr| addr.is_some())
          .map(|addr| addr.unwrap())
          .collect::<Vec<String>>()
          .join(", ")
      )
  }

  fn extract_group_value(&self, group: &Group) -> Option<String> {
    let name = group.name.as_ref().map(|s| s.to_string());
    let addresses = self.extract_address_list_value(&group.addresses);
    self.format_address(&name, &addresses)
  }

  fn extract_group_list_value(&self, group_list: &Vec<Group>) -> Option<String> {
    (!group_list.is_empty())
      .then(||
        group_list.iter()
          .map(|group| self.extract_group_value(&group))
          .filter(|group| group.is_some())
          .map(|group| group.unwrap())
          .collect::<Vec<String>>()
          .join(", ")
      )
  }

  fn extract_text_list_value(&self, name: &str, text_list: &Vec<Cow<str>>) -> Option<String> {
    (!text_list.is_empty())
      .then(||
        text_list.iter()
          .map(|text| self.extract_text_value(name, text))
          .filter(|text| text.is_some())
          .map(|text| text.unwrap())
          .collect::<Vec<String>>()
          .join(", ")
      )
  }

  fn extract_text_value(&self, name: &str, text: &Cow<str>) -> Option<String> {
    Some(format!("<b>{}</b>: {}", name, text))
  }
}

impl MessageVisitor for HtmlMessageVisitor {
  fn on_head_body_separator(&self) -> Option<String> {
    Some("<br>\n".to_string())
  }

  fn on_header_prefix(&self) -> Option<String> {
    Some("<div>".to_string())
  }

  fn on_header_suffix(&self) -> Option<String> {
    Some("</div>".to_string())
  }

  fn on_part_prefix(&self) -> Option<String> {
    Some("<div>".to_string())
  }

  fn on_part_suffix(&self) -> Option<String> {
    Some("</div>".to_string())
  }

  fn on_header_address(&self, header_name: &str, address: &Addr) -> Option<String> {
    self.extract_address_value(address)
      .map(|value| format!("<b>{}</b>: {}", header_name, value))
  }

  fn on_header_address_list(&self, name: &str, address_list: &Vec<Addr>) -> Option<String> {
    self.extract_address_list_value(address_list)
      .map(|addrs| format!("<b>{}</b>: {}", name, addrs))
  }

  fn on_header_group(&self, header_name: &str, group: &Group) -> Option<String> {
    self.extract_group_value(group)
      .map(|value| format!("<b>{}</b>: {}", header_name, value))
  }

  fn on_header_group_list(&self, name: &str, group_list: &Vec<Group>) -> Option<String> {
    self.extract_group_list_value(group_list)
      .map(|groups| format!("<b>{}</b>: {}", name, groups))
  }

  fn on_header_text(&self, name: &str, text: &Cow<str>) -> Option<String> {
    self.extract_text_value(name, text)
  }

  fn on_header_text_list(&self, name: &str, text_list: &Vec<Cow<str>>) -> Option<String> {
    self.extract_text_list_value(name, text_list)
      .map(|texts| format!("<b>{}</b>: {}", name, texts))
  }

  fn on_header_date_time(&self, name: &str, date_time: &DateTime) -> Option<String> {
    Some(format!("<b>{}</b>: {}", name, date_time))
  }

  fn on_header_content_type(&self, _: &ContentType) -> Option<String> {
    None
  }

  fn on_part_text(&self, value: &Cow<str>) -> String {
    value.to_string()
  }

  fn on_part_html(&self, value: &Cow<str>) -> String {
    value.to_string()
  }

  fn on_part_binary(&self, value: &Cow<[u8]>) -> Vec<u8> {
    value.to_vec()
  }

  fn on_part_inline_binary(&self, value: &Cow<[u8]>) -> Vec<u8> {
    value.to_vec()
  }
}
