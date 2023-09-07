use std::io::Write;

use mail_parser::{HeaderValue, Message, MessagePart, PartType};

use crate::message::rfc822::message_visitor::MessageVisitor;

/// Service to transform message content using a provided visitor implementation.
///
pub struct MessageTransformer {
    visitor: Box<dyn MessageVisitor>,
}

impl MessageTransformer {
    /// Creates a new transformer that will use the provided visitor to transform the message.
    ///
    pub fn new(visitor: Box<dyn MessageVisitor>) -> Self {
        Self { visitor }
    }

    /// Transforms the message and writes the result to the provided writer.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to transform.
    /// * `writer` - The writer to write the transformed message to.
    ///
    pub fn transform<W>(&self, message: &Message, writer: &mut W) -> anyhow::Result<()>
    where
        W: Write,
    {
        for header in message.headers() {
            if let Some(header_value) = self.transform_header(header.name(), header.value()) {
                self.write_if_some(writer, self.visitor.on_header_prefix())?;

                let value = format!("{}", header_value);
                writer.write_all(value.as_bytes())?;

                self.write_if_some(writer, self.visitor.on_header_suffix())?;
                writer.write_all(b"\n")?;
            }
        }

        self.write_if_some(writer, self.visitor.on_head_body_separator())?;

        let bodies = if message.html_body_count() > 0 {
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

    /// Transforms the message header value identified by the provided name.
    ///
    fn transform_header(&self, name: &str, value: &HeaderValue) -> Option<String> {
        match value {
            HeaderValue::Address(addr) => self.visitor.on_header_address(name, addr),

            HeaderValue::AddressList(addr_list) => {
                self.visitor.on_header_address_list(name, addr_list)
            }

            HeaderValue::Group(group) => self.visitor.on_header_group(name, group),

            HeaderValue::GroupList(group_list) => {
                self.visitor.on_header_group_list(name, group_list)
            }

            HeaderValue::Text(text) => self.visitor.on_header_text(name, text),

            HeaderValue::TextList(text_list) => self.visitor.on_header_text_list(name, text_list),

            HeaderValue::DateTime(date_time) => {
                println!("{}", date_time.to_string());
                self.visitor.on_header_date_time(name, date_time)
            }

            HeaderValue::ContentType(content_type) => {
                self.visitor.on_header_content_type(&content_type)
            }

            HeaderValue::Empty => None,
        }
    }

    /// Transforms the provided message part and writes the result to the provided writer.
    ///
    fn transform_part<W>(
        &self,
        message: &Message,
        writer: &mut W,
        part: &MessagePart,
    ) -> anyhow::Result<()>
    where
        W: Write,
    {
        match &part.body {
            PartType::Text(text) => {
                let text = self.visitor.on_part_text(text);
                writer.write_all(text.as_bytes())?;
            }

            PartType::Html(html) => {
                let html = self.visitor.on_part_html(html);
                writer.write_all(html.as_bytes())?;
            }

            PartType::Binary(binary) => {
                let binary = self.visitor.on_part_binary(binary);
                writer.write_all(binary.as_ref())?;
            }

            PartType::InlineBinary(inline_binary) => {
                let inline_binary = self.visitor.on_part_inline_binary(inline_binary);
                writer.write_all(inline_binary.as_ref())?;
            }

            PartType::Message(message) => self.transform(message, writer)?,

            PartType::Multipart(part_ids) => {
                for part_id in part_ids {
                    if let Some(p) = message.part(*part_id) {
                        self.transform_part(message, writer, p)?;
                    }
                }
            }
        };

        Ok(())
    }

    /// Writes the provided value to the writer if it is not `None`.
    ///
    fn write_if_some<W>(&self, writer: &mut W, value: Option<String>) -> anyhow::Result<()>
    where
        W: Write,
    {
        if let Some(value) = value {
            writer.write_all(value.as_bytes())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::borrow::Cow;

    use crate::test_util;
    use anyhow::anyhow;
    use mail_parser::{Addr, ContentType, DateTime, Group};

    use super::*;

    struct TestVisitor;

    impl MessageVisitor for TestVisitor {
        fn on_header_address<'a>(&'a self, name: &str, address: &Addr<'a>) -> Option<String> {
            match name {
                "From" => {
                    assert_eq!(
                        &Addr {
                            name: None,
                            address: Some(Cow::from("rusty.processing@mime.com"))
                        },
                        address
                    );
                    Some("From header".to_string())
                }
                "To" => {
                    assert_eq!(
                        &Addr {
                            name: None,
                            address: Some(Cow::from("processing.rusty@emim.com"))
                        },
                        address
                    );
                    Some("To header".to_string())
                }
                _ => panic!("Unexpected header: {}", name),
            }
        }

        fn on_header_address_list<'a>(
            &self,
            name: &str,
            address_list: &Vec<Addr<'a>>,
        ) -> Option<String> {
            panic!("Unexpected header: ({}, {:?})", name, address_list)
        }

        fn on_header_group<'a>(&self, name: &str, group: &Group<'a>) -> Option<String> {
            panic!("Unexpected header: ({}, {:?})", name, group)
        }

        fn on_header_group_list<'a>(
            &self,
            name: &str,
            group_list: &Vec<Group<'a>>,
        ) -> Option<String> {
            panic!("Unexpected header: ({}, {:?})", name, group_list)
        }

        fn on_header_text<'a>(&self, name: &str, text: &Cow<'a, str>) -> Option<String> {
            match name {
                "Message-ID" => {
                    assert_eq!("12345-headers-small@rusty-processing", text);
                    Some("Message-ID header".to_string())
                }
                "Subject" => {
                    assert_eq!("Now THATS A LOT OF RUST", text);
                    Some("Subject header".to_string())
                }
                "MIME-Version" => {
                    assert_eq!("1.0", text);
                    Some("Mime-Version header".to_string())
                }
                "Content-Transfer-Encoding" => {
                    assert_eq!("7bit", text);
                    Some("Content-Transfer-Encoding header".to_string())
                }
                _ => panic!("Unexpected header: {}", name),
            }
        }

        fn on_header_text_list<'a>(
            &self,
            name: &str,
            text_list: &Vec<Cow<'a, str>>,
        ) -> Option<String> {
            panic!("Unexpected header: ({}, {:?})", name, text_list)
        }

        fn on_header_date_time(&self, _name: &str, date_time: &DateTime) -> Option<String> {
            assert_eq!("2021-02-21T07:58:00-08:00", date_time.to_string());
            Some("Date header".to_string())
        }

        fn on_header_content_type<'a>(&self, content_type: &ContentType<'a>) -> Option<String> {
            assert_eq!(
                &ContentType {
                    c_type: Cow::from("text"),
                    c_subtype: Some(Cow::from("plain")),
                    attributes: Some(vec![(Cow::from("charset"), Cow::from("us-ascii"))])
                },
                content_type
            );
            Some("Content-Type header".to_string())
        }

        fn on_part_text<'a>(&self, value: &Cow<'a, str>) -> String {
            assert_eq!("This is a rusty email\n\n;)\n", value);
            "Text part".to_string()
        }

        fn on_part_html<'a>(&self, value: &Cow<'a, str>) -> String {
            panic!("Unexpected part: {}", value)
        }

        fn on_part_binary<'a>(&self, value: &Cow<'a, [u8]>) -> Vec<u8> {
            panic!("Unexpected part: {:?}", value)
        }

        fn on_part_inline_binary<'a>(&self, value: &Cow<'a, [u8]>) -> Vec<u8> {
            panic!("Unexpected part: {:?}", value)
        }
    }

    #[test]
    fn test_transform() -> anyhow::Result<()> {
        let content = test_util::read_contents("resources/rfc822/headers-small.eml")?;
        let message = Message::parse(&content).ok_or(anyhow!("Failed to parse message"))?;
        let transformer = MessageTransformer::new(Box::new(TestVisitor {}));

        let mut content = vec![];
        transformer.transform(&message, &mut content)?;

        let expected_content = "\
Message-ID header
Date header
From header
To header
Subject header
Mime-Version header
Content-Type header
Content-Transfer-Encoding header
Text part";

        assert_eq!(expected_content, String::from_utf8(content)?);
        Ok(())
    }
}
