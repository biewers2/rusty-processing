use std::borrow::Cow;

use html_escape::encode_text;
use mail_parser::{Addr, ContentType, DateTime, Group};

use crate::message::rfc822::message_formatter::MessageFormatter;
use crate::message::rfc822::message_visitor::MessageVisitor;

const HEADERS: [&str; 6] = ["Date", "From", "To", "CC", "BCC", "Subject"];

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
        self.formatter
            .format_address(address)
            .map(|value| format!("<b>{}</b>: {}", header_name, encode_text(value.as_str())))
    }

    fn on_header_address_list(&self, name: &str, address_list: &Vec<Addr>) -> Option<String> {
        self.formatter
            .format_address_list(address_list)
            .map(|addrs| format!("<b>{}</b>: {}", name, encode_text(addrs.as_str())))
    }

    fn on_header_group(&self, header_name: &str, group: &Group) -> Option<String> {
        self.formatter
            .format_group(group)
            .map(|value| format!("<b>{}</b>: {}", header_name, encode_text(value.as_str())))
    }

    fn on_header_group_list(&self, name: &str, group_list: &Vec<Group>) -> Option<String> {
        self.formatter
            .format_group_list(group_list)
            .map(|groups| format!("<b>{}</b>: {}", name, encode_text(groups.as_str())))
    }

    fn on_header_text(&self, name: &str, text: &Cow<str>) -> Option<String> {
        HEADERS
            .contains(&name)
            .then_some(format!("<b>{}</b>: {}", name, encode_text(text)))
    }

    fn on_header_text_list(&self, name: &str, text_list: &Vec<Cow<str>>) -> Option<String> {
        self.formatter
            .format_text_list(text_list)
            .map(|texts| format!("<b>{}</b>: {}", name, encode_text(texts.as_str())))
    }

    fn on_header_date_time(&self, name: &str, date_time: &DateTime) -> Option<String> {
        Some(format!(
            "<b>{}</b>: {}",
            name,
            encode_text(date_time.to_string().as_str())
        ))
    }

    fn on_header_content_type(&self, _: &ContentType) -> Option<String> {
        None
    }

    fn on_part_text(&self, value: &Cow<str>) -> String {
        value
            .split("\n")
            .map(|line| format!("<p>{}</p>", encode_text(line)))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

#[cfg(test)]
mod test {
    use anyhow::anyhow;
    use mail_parser::Message;

    use crate::message::rfc822::transformer::MessageTransformer;
    use crate::test_util;

    use super::*;

    #[test]
    fn test_text_message_visitor() -> anyhow::Result<()> {
        let content = test_util::read_contents("resources/rfc822/headers-small.eml")?;
        let message = Message::parse(&content).ok_or(anyhow!("Failed to parse message"))?;
        let visitor = Box::<HtmlMessageVisitor>::default();
        let transformer = MessageTransformer::new(visitor);

        let mut content = vec![];
        transformer.transform(&message, &mut content)?;

        let expected_content = "\
<div><b>Date</b>: 2021-02-21T07:58:00-08:00</div>
<div><b>From</b>: &lt;rusty.processing@mime.com&gt;</div>
<div><b>To</b>: &lt;processing.rusty@emim.com&gt;</div>
<div><b>Subject</b>: Now THATS A LOT OF RUST</div>
<br>
<div><p>This is a rusty email</p>
<p></p>
<p>;)</p>
<p></p></div>";
        assert_eq!(expected_content, String::from_utf8(content)?);
        Ok(())
    }
}
