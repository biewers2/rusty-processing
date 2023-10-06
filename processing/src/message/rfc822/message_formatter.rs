use std::borrow::Cow;

use mail_parser::{Addr, Group};

/// A formatter for RFC 822 messages.
///
/// This formatter provides a default implementation used to format `mail-parser` header values of RFC822 messages.
///
#[derive(Default)]
pub struct MessageFormatter {}

impl MessageFormatter {
    /// Formats an `Addr` into an optional `String`.
    ///
    /// Here are the formatting rules:
    /// 1. Name AND Address -> Some("Name &lt;Address&gt;")
    /// 2. Name -> Some("Name")
    /// 3. Address -> Some("&lt;Address&gt;")
    /// 4. Neither -> None
    ///
    pub fn format_address(&self, address: &Addr) -> Option<String> {
        let name = address.name.as_ref().map(|s| s.to_string());
        let address = address.address.as_ref().map(|s| s.to_string());
        self.format_name_address(&name, &address)
    }

    /// Formats a list of `Addr` into an optional `String`.
    ///
    /// The address are formatted using `format_address` and then concatenated into a single string separated by ", ".
    ///
    pub fn format_addresses(&self, addresses: &[Addr]) -> Option<String> {
        (!addresses.is_empty())
            .then(|| {
                addresses
                    .iter()
                    .filter_map(|addr| self.format_address(addr))
                    .collect::<Vec<String>>()
                    .join(", ")
            })
            .and_then(|value| (!value.is_empty()).then_some(value))
    }

    /// Formats a `Group` into an optional `String`.
    ///
    /// The group is formatted using `format_address_list` for the addresses, and then `format_name_address` for the name and the result
    /// of the address list formatting.
    ///
    /// ### Example Output
    ///
    /// ```text
    /// "GroupName <Name1 <address1@domain1>, Name2 <address2@domain2>, ...>"
    /// ```
    ///
    pub fn format_group(&self, group: &Group) -> Option<String> {
        let name = group.name.as_ref().map(|s| s.to_string());
        let addresses = self.format_addresses(&group.addresses);
        self.format_name_address(&name, &addresses)
    }

    /// Formats a list of `Group` into an optional `String`.
    ///
    /// The groups are formatted using `format_group` and then concatenated into a single string separated by ", ".
    ///
    /// ### Example Output
    ///
    /// ```text
    /// "GroupName1 <Name1 <address1@domain1>, ...>, GroupName2 <Name1 <address1@domain1>, ...>, ...>"
    /// ```
    pub fn format_groups(&self, groups: &[Group]) -> Option<String> {
        (!groups.is_empty())
            .then(|| {
                groups
                    .iter()
                    .filter_map(|group| self.format_group(group))
                    .collect::<Vec<String>>()
                    .join(", ")
            })
            .and_then(|value| (!value.is_empty()).then_some(value))
    }

    /// Formats a list of `String` into an optional `String`.
    ///
    /// The strings are concatenated into a single string separated by ", ".
    /// If the list is empty, `None` is returned.
    ///
    pub fn format_text_list(&self, text_list: &[Cow<str>]) -> Option<String> {
        let list: Vec<Cow<str>> = text_list.iter()
            .cloned()
            .filter(|text| !text.is_empty())
            .collect();
        (!list.is_empty()).then(|| list.join(", "))
    }

    /// Formats a name and an address into an optional `String`.
    ///
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

#[cfg(test)]
mod test {
    use super::*;

    fn addr<'a>(name: &'a str, address: &'a str) -> Addr<'a> {
        Addr {
            name: (!name.is_empty()).then_some(Cow::from(name)),
            address: (!address.is_empty()).then_some(Cow::from(address)),
        }
    }

    fn group<'a>(name: &'a str, addresses: Vec<Addr<'a>>) -> Group<'a> {
        Group {
            name: (!name.is_empty()).then_some(Cow::from(name)),
            addresses,
        }
    }

    #[test]
    fn test_format_address() -> anyhow::Result<()> {
        let formatter = MessageFormatter::default();
        let cases = vec![
            (
                addr("name", "name@domain.com"),
                Some("name <name@domain.com>".to_string()),
            ),
            (addr("name-only", ""), Some("name-only".to_string())),
            (addr("", "address-only"), Some("<address-only>".to_string())),
            (addr("", ""), None),
        ];

        for (addr, expected) in cases {
            let actual = formatter.format_address(&addr);
            assert_eq!(expected, actual);
        }

        Ok(())
    }

    #[test]
    fn test_format_address_list() -> anyhow::Result<()> {
        let formatter = MessageFormatter::default();
        let cases = vec![
            (
                vec![
                    addr("name", "name@domain.com"),
                    addr("name2", "name2@domain.com"),
                    addr("abcde", "abcde@email.com"),
                ],
                Some(
                    "name <name@domain.com>, name2 <name2@domain.com>, abcde <abcde@email.com>"
                        .to_string(),
                ),
            ),
            (
                vec![
                    addr("name", ""),
                    addr("", ""),
                    addr("abcde", "abcde@email.com"),
                ],
                Some("name, abcde <abcde@email.com>".to_string()),
            ),
            (vec![addr("", ""), addr("", "")], None),
        ];

        for (addrs, expected) in cases {
            let actual = formatter.format_addresses(&addrs);
            assert_eq!(expected, actual);
        }

        Ok(())
    }

    #[test]
    fn test_format_group() -> anyhow::Result<()> {
        let formatter = MessageFormatter::default();
        let cases = vec![
            (
                group(
                    "group-name",
                    vec![
                        addr("name", "name@domain.com"),
                        addr("name2", "name2@email.com"),
                    ],
                ),
                Some("group-name <name <name@domain.com>, name2 <name2@email.com>>".to_string()),
            ),
            (
                group("", vec![addr("", "name@domain.com")]),
                Some("<<name@domain.com>>".to_string()),
            ),
            (
                group("group-name-only", vec![]),
                Some("group-name-only".to_string()),
            ),
            (group("", vec![]), None),
        ];

        for (group, expected) in cases {
            let actual = formatter.format_group(&group);
            assert_eq!(expected, actual);
        }

        Ok(())
    }

    #[test]
    fn test_format_group_list() -> anyhow::Result<()> {
        let formatter = MessageFormatter::default();
        let cases = vec![
            (
                vec![
                    group("group-name", vec![
                        addr("name", "name@domain.com"),
                        addr("name2", "name2@email.com"),
                    ]),
                    group("gn2", vec![
                        addr("", "name@domain.com"),
                    ]),
                ],
                Some("group-name <name <name@domain.com>, name2 <name2@email.com>>, gn2 <<name@domain.com>>".to_string())
            ),
            (
                vec![
                    group("", vec![]),
                    group("", vec![addr("", "")]),
                ],
                None
            ),
        ];

        for (groups, expected) in cases {
            let actual = formatter.format_groups(&groups);
            assert_eq!(expected, actual);
        }

        Ok(())
    }

    #[test]
    fn test_format_text_list() -> anyhow::Result<()> {
        let formatter = MessageFormatter::default();
        let cases = vec![
            (
                vec![Cow::from("text1"), Cow::from("text2"), Cow::from("text3")],
                Some("text1, text2, text3".to_string()),
            ),
            (vec![Cow::from(""), Cow::from("")], None),
        ];

        for (texts, expected) in cases {
            let actual = formatter.format_text_list(&texts);
            assert_eq!(expected, actual);
        }

        Ok(())
    }
}
