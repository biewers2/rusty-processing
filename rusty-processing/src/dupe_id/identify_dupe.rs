use crate::dupe_id::md5_dupe_identifier::md5_dupe_identifier;
use crate::dupe_id::message_dupe_identifier::message_dupe_identifier;

/// Defines a type for a boxed [`IdentifyDupe`] implementation.
///
pub type IdentifyDupeService = Box<dyn IdentifyDupe>;

/// Defines the interface for a duplicate file identification service.
///
pub trait IdentifyDupe: Send + Sync {
    /// Identifies duplicate file by producing a unique identifier for the file.
    ///
    fn identify(&self, raw: &[u8]) -> String;
}

pub fn identifier(mimetype: &String) -> &'static IdentifyDupeService {
    match mimetype.as_str() {
        "message/rfc822" => message_dupe_identifier(),
        _ => md5_dupe_identifier(),
    }
}
