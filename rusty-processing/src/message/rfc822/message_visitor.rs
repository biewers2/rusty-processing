use std::borrow::Cow;

use mail_parser::{Addr, ContentType, DateTime, Group, Received};

pub trait MessageVisitor {
    fn on_header_prefix(&self) -> Option<String> {
        None
    }

    fn on_header_suffix(&self) -> Option<String> {
        None
    }

    fn on_head_body_separator(&self) -> Option<String> {
        None
    }

    fn on_part_prefix(&self) -> Option<String> {
        None
    }

    fn on_part_suffix(&self) -> Option<String> {
        None
    }

    // Header visitors

    fn on_header_received<'a>(&self, _name: &str, _received: &Received<'a>) -> Option<String> {
        None
    }

    fn on_header_addresses<'a>(
        &self,
        _name: &str,
        _address_list: &Vec<Addr<'a>>,
    ) -> Option<String> {
        None
    }

    fn on_header_groups<'a>(
        &self,
        _name: &str,
        _group_list: &Vec<Group<'a>>,
    ) -> Option<String> {
        None
    }

    fn on_header_text<'a>(&self, _name: &str, _text: &Cow<'a, str>) -> Option<String> {
        None
    }

    fn on_header_text_list<'a>(
        &self,
        _name: &str,
        _text_list: &Vec<Cow<'a, str>>,
    ) -> Option<String> {
        None
    }

    fn on_header_date_time(&self, _name: &str, _date_time: &DateTime) -> Option<String> {
        None
    }

    fn on_header_content_type<'a>(&self, _content_type: &ContentType<'a>) -> Option<String> {
        None
    }

    // Body part visitors

    fn on_part_text<'a>(&self, value: &Cow<'a, str>) -> String {
        value.to_string()
    }

    fn on_part_html<'a>(&self, value: &Cow<'a, str>) -> String {
        value.to_string()
    }

    fn on_part_binary<'a>(&self, value: &Cow<'a, [u8]>) -> Vec<u8> {
        value.to_vec()
    }

    fn on_part_inline_binary<'a>(&self, value: &Cow<'a, [u8]>) -> Vec<u8> {
        value.to_vec()
    }
}
