use lazy_static::lazy_static;

use crate::{Identify, IdentifyDedupeService};

lazy_static! {
    static ref MD5_DEDUPE_IDENTIFIER: IdentifyDedupeService = Box::<Md5DedupeIdentifier>::default();
}

/// Returns a reference to the MD5 dupe identifier service singleton.
///
pub fn md5_dedupe_identifier() -> &'static IdentifyDedupeService {
    &MD5_DEDUPE_IDENTIFIER
}

/// MD5 dupe identifier.
///
/// This identifier uses the MD5 hash of the raw bytes to identify duplicates.
///
#[derive(Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Md5DedupeIdentifier;

impl Identify for Md5DedupeIdentifier {
    fn identify(&self, raw: &[u8]) -> String {
        format!("{:x}", md5::compute(raw))
    }
}

#[cfg(test)]
mod tests {
    use std::any::{Any, TypeId};

    use super::*;

    #[test]
    fn check_md5_dedupe_identifier_singleton() {
        assert_eq!(
            md5_dedupe_identifier().type_id(),
            TypeId::of::<IdentifyDedupeService>()
        );
    }
}
