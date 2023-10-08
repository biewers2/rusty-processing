use std::borrow::Cow;

use mail_parser::{Addr, ContentType, DateTime, Group};

use crate::message::rfc822::message_formatter::MessageFormatter;
use crate::message::rfc822::message_visitor::MessageVisitor;

const HEADERS: [&str; 6] = ["Date", "From", "To", "CC", "BCC", "Subject"];

#[derive(Default)]
pub struct TextMessageVisitor {
    formatter: MessageFormatter,
}

impl MessageVisitor for TextMessageVisitor {
    fn on_head_body_separator(&self) -> Option<String> {
        Some("\n".to_string())
    }

    // fn on_header_received<'a>(&self, _name: &str, _received: &Received<'a>) -> Option<String> {
    //     todo!()
    // }

    fn on_header_addresses(&self, name: &str, address_list: &[Addr]) -> Option<String> {
        self.formatter
            .format_addresses(address_list)
            .map(|addrs| format!("{}: {}", name, addrs))
    }

    fn on_header_groups(&self, name: &str, group_list: &[Group]) -> Option<String> {
        self.formatter
            .format_groups(group_list)
            .map(|groups| format!("{}: {}", name, groups))
    }

    fn on_header_text(&self, name: &str, text: Cow<str>) -> Option<String> {
        HEADERS
            .contains(&name)
            .then_some(format!("{}: {}", name, text))
    }

    fn on_header_text_list(&self, name: &str, text_list: &[Cow<str>]) -> Option<String> {
        self.formatter
            .format_text_list(text_list)
            .map(|texts| format!("{}: {}", name, texts))
    }

    fn on_header_date_time(&self, name: &str, date_time: &DateTime) -> Option<String> {
        Some(format!("{}: {}", name, date_time))
    }

    fn on_header_content_type(&self, _: &ContentType) -> Option<String> {
        None
    }

    fn on_part_html(&self, value: Cow<str>) -> String {
        html2text::from_read(value.as_bytes(), 120)
    }
}

#[cfg(test)]
mod test {
    use anyhow::anyhow;
    use mail_parser::MessageParser;
    use test_utils::read_contents;

    use crate::message::rfc822::transformer::MessageTransformer;

    use super::*;

    #[test]
    fn test_text_message_visitor() -> anyhow::Result<()> {
        let content = read_contents("../resources/rfc822/headers-small.eml")?;
        let message = MessageParser::default().parse(&content).ok_or(anyhow!("Failed to parse message"))?;
        let visitor = Box::<TextMessageVisitor>::default();
        let transformer = MessageTransformer::new(visitor);

        let mut content = vec![];
        transformer.transform(&message, &mut content)?;

        let expected_content = "\
Date: 2021-02-21T07:58:00-08:00
From: <rusty.processing@mime.com>
To: <processing.rusty@emim.com>
Subject: Now THATS A LOT OF RUST

This is a rusty email

;)
";
        assert_eq!(expected_content, String::from_utf8(content)?);
        Ok(())
    }
}
