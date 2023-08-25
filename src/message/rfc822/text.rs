use std::borrow::Cow;
use std::fmt::format;
use std::fs::File;
use std::path;

use mail_parser::{Addr, ContentType, DateTime, Group, Message, MessagePartId};

use crate::common::error::{ProcessError, ProcessResult};
use crate::message::rfc822::html_visitor::HtmlMessageVisitor;
use crate::message::rfc822::transformer::MessageTransformer;
use crate::message::rfc822::visitor::MessageVisitor;

#[derive(Default)]
struct TextExtractingMessageVisitor {}

// impl MessageVisitor for TextExtractingMessageVisitor {
//   fn on_head_body_separator(&self) -> Option<String> {
//     Some("\n".to_string())
//   }
//
//   fn on_header_address(&self, header_name: &str, address: &Addr) -> Option<String> {
//     let name = address.name.as_ref().map(|s| s.to_string());
//     let address = address.address.as_ref().map(|s| s.to_string());
//
//     match (name, address) {
//       (Some(name), Some(address)) => Some(format!("{} <{}>", name, address)),
//       (Some(name), None) => Some(name),
//       (None, Some(address)) => Some(format!("<{}>", address)),
//       (None, None) => None,
//     }.map(|value|
//       format!("{}: {}", header_name, value)
//     )
//   }
//
//   fn on_header_address_list(&self, name: &str, address_list: &Vec<Addr>) -> Option<String> {
//     (!address_list.is_empty())
//       .then(||
//         address_list.iter()
//           .map(|addr| self.transform_header_address(&addr))
//           .filter(|addr| addr.is_some())
//           .map(|addr| addr.unwrap())
//           .collect::<Vec<String>>()
//           .join(", ")
//       )
//       .map(|addrs| format!("{}: {}", name, addrs))
//   }
//
//   fn on_header_group(&self, header_name: &str, group: &Group) -> Option<String> {
//     let name = group.name.as_ref().map(|s| s.to_string());
//     let addresses = self.transform_header_address_list(&group.addresses);
//
//     match (name, addresses) {
//       (Some(name), Some(addresses)) => Some(format!("{}: <{}>", name, addresses)),
//       (Some(name), None) => Some(name),
//       (None, Some(addresses)) => Some(format!("<{}>", addresses)),
//       (None, None) => None,
//     }.map(|value|
//       format!("{}: {}", header_name, value)
//     )
//   }
//
//   fn on_header_group_list(&self, name: &str, group_list: &Vec<Group>) -> Option<String> {
//     (!group_list.is_empty())
//       .then(||
//         group_list.iter()
//           .map(|group| self.transform_header_group(&group))
//           .filter(|group| group.is_some())
//           .map(|group| group.unwrap())
//           .collect::<Vec<String>>()
//           .join(", ")
//       )
//       .map(|groups| format!("{}: {}", name, groups))
//   }
//
//   fn on_header_text(&self, name: &str, text: &Cow<str>) -> Option<String> {
//     Some(format!("{}: {}", name, text))
//   }
//
//   fn on_header_text_list(&self, name: &str, text_list: &Vec<Cow<str>>) -> Option<String> {
//     (!text_list.is_empty())
//       .then(||
//         text_list.iter()
//           .map(|text| self.on_header_text(name, text))
//           .filter(|text| text.is_some())
//           .map(|text| text.unwrap())
//           .collect::<Vec<String>>()
//           .join(", ")
//       )
//       .map(|texts| format!("{}: {}", name, texts))
//   }
//
//   fn on_header_date_time(&self, name: &str, date_time: &DateTime) -> Option<String> {
//     Some(format!("{}: {}", name, date_time))
//   }
//
//   fn on_header_content_type(&self, text: &ContentType) -> Option<String> {
//     None
//   }
//
//   fn on_part_text(&self, value: &Cow<str>) -> String {
//     value.to_string()
//   }
//
//   fn on_part_html(&self, value: &Cow<str>) -> String {
//   }
//
//   fn on_part_binary(&self, value: &Cow<[u8]>) -> &[u8] {
//     value
//   }
//
//   fn on_part_inline_binary(&self, value: &Cow<[u8]>) -> &[u8] {
//     value
//   }
// }

pub fn extract(message: &Message, output_path: path::PathBuf) -> ProcessResult<()> {
  let transformer = MessageTransformer::new(Box::<HtmlMessageVisitor>::default());
  let mut file = File::create(output_path).map_err(ProcessError::Io)?;

  transformer.transform(message, &mut file)
}