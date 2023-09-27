use std::borrow::Cow;

use lazy_static::lazy_static;
use mail_parser::MessageParser;
use uuid::Uuid;

use crate::{Identify, IdentifyDedupeService};
use crate::md5_dedupe_identifier;

lazy_static! {
    static ref MESSAGE_DUPE_IDENTIFIER: IdentifyDedupeService =
        Box::<MessageDedupeIdentifier>::default();
}

/// Returns a reference to the message dupe identifier service singleton.
pub fn message_dedupe_identifier() -> &'static IdentifyDedupeService {
    &MESSAGE_DUPE_IDENTIFIER
}

/// Identifies a message by its message ID, or if it doesn't have one, by a
/// randomly generated UUID.
///
#[derive(Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct MessageDedupeIdentifier;

impl Identify for MessageDedupeIdentifier {
    fn identify(&self, raw: &[u8]) -> String {
        let message = MessageParser::default().parse(raw);
        let raw_id = message
            .as_ref()
            .and_then(|msg| msg.message_id())
            .map(|id| Cow::from(id.as_bytes()))
            .unwrap_or_else(|| Cow::from(Uuid::new_v4().as_ref().to_owned()));

        md5_dedupe_identifier().identify(&raw_id)
    }
}

#[cfg(test)]
mod tests {
    use std::any::{Any, TypeId};

    use crate::identify::IdentifyDedupeService;

    use super::*;

    #[test]
    fn check_message_dupe_identifier_singleton() {
        assert_eq!(
            message_dedupe_identifier().type_id(),
            TypeId::of::<IdentifyDedupeService>()
        );
    }
}
