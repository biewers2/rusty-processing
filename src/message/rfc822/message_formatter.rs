use std::borrow::Cow;

use mail_parser::{Addr, Group};

#[derive(Default)]
pub struct MessageFormatter {}

impl MessageFormatter {
    pub fn format_address(&self, address: &Addr) -> Option<String> {
        let name = address.name.as_ref().map(|s| s.to_string());
        let address = address.address.as_ref().map(|s| s.to_string());
        self.format_name_address(&name, &address)
    }

    pub fn format_address_list(&self, address_list: &Vec<Addr>) -> Option<String> {
        (!address_list.is_empty()).then(|| {
            address_list
                .iter()
                .map(|addr| self.format_address(&addr))
                .filter(|addr| addr.is_some())
                .map(|addr| addr.unwrap())
                .collect::<Vec<String>>()
                .join(", ")
        })
    }

    pub fn format_group(&self, group: &Group) -> Option<String> {
        let name = group.name.as_ref().map(|s| s.to_string());
        let addresses = self.format_address_list(&group.addresses);
        self.format_name_address(&name, &addresses)
    }

    pub fn format_group_list(&self, group_list: &Vec<Group>) -> Option<String> {
        (!group_list.is_empty()).then(|| {
            group_list
                .iter()
                .map(|group| self.format_group(&group))
                .filter(|group| group.is_some())
                .map(|group| group.unwrap())
                .collect::<Vec<String>>()
                .join(", ")
        })
    }

    pub fn format_text_list(&self, text_list: &Vec<Cow<str>>) -> Option<String> {
        (!text_list.is_empty()).then(|| text_list.join(", "))
    }

    fn format_name_address(
        &self,
        name: &Option<String>,
        address: &Option<String>,
    ) -> Option<String> {
        match (name, address) {
            (Some(name), Some(address)) => Some(format!("{} <{}>", name, address)),
            (Some(name), None) => Some(name.to_string()),
            (None, Some(address)) => Some(format!("<{}>", address)),
            (None, None) => None,
        }
    }
}
