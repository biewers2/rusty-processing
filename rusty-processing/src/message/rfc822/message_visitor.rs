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

    fn on_header_received(&self, _name: &str, _received: &Received<'_>) -> Option<String> {
        None
    }

    fn on_header_addresses(
        &self,
        _name: &str,
        _address_list: &Vec<Addr<'_>>,
    ) -> Option<String> {
        None
    }

    fn on_header_groups(
        &self,
        _name: &str,
        _group_list: &Vec<Group<'_>>,
    ) -> Option<String> {
        None
    }

    fn on_header_text(&self, _name: &str, _text: &Cow<'_, str>) -> Option<String> {
        None
    }

    fn on_header_text_list(
        &self,
        _name: &str,
        _text_list: &Vec<Cow<'_, str>>,
    ) -> Option<String> {
        None
    }

    fn on_header_date_time(&self, _name: &str, _date_time: &DateTime) -> Option<String> {
        None
    }

    fn on_header_content_type(&self, _content_type: &ContentType<'_>) -> Option<String> {
        None
    }

    // Body part visitors

    fn on_part_text(&self, value: &Cow<'_, str>) -> String {
        value.to_string()
    }

    fn on_part_html(&self, value: &Cow<'_, str>) -> String {
        value.to_string()
    }

    fn on_part_binary(&self, value: &Cow<'_, [u8]>) -> Vec<u8> {
        value.to_vec()
    }

    fn on_part_inline_binary(&self, value: &Cow<'_, [u8]>) -> Vec<u8> {
        value.to_vec()
    }
}
