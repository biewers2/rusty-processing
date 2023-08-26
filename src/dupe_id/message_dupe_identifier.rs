use crate::dupe_id::identify_dupe::{IdentifyDupe, IdentifyDupeService};
use crate::dupe_id::md5_dupe_identifier::md5_dupe_identifier;
use lazy_static::lazy_static;
use mail_parser::Message;
use std::borrow::Cow;
use uuid::Uuid;

lazy_static! {
    static ref MESSAGE_DUPE_IDENTIFIER: IdentifyDupeService =
        Box::<MessageDupeIdentifier>::default();
}

/// Returns a reference to the message dupe identifier service singleton.
pub fn message_dupe_identifier() -> &'static IdentifyDupeService {
    &MESSAGE_DUPE_IDENTIFIER
}

/// Identifies a message by its message ID, or if it doesn't have one, by a
/// randomly generated UUID.
///
#[derive(Default)]
pub struct MessageDupeIdentifier {}

impl IdentifyDupe for MessageDupeIdentifier {
    fn identify(&self, raw: &[u8]) -> String {
        let message = Message::parse(&raw);
        let raw_id = message
            .as_ref()
            .and_then(|msg| msg.message_id())
            .map(|id| Cow::from(id.as_bytes()))
            .unwrap_or_else(|| Cow::from(Uuid::new_v4().as_ref().to_owned()));

        md5_dupe_identifier().identify(&raw_id)
    }
}
