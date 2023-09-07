use lazy_static::lazy_static;

use crate::identify::{Identify, IdentifyDupeService};

lazy_static! {
    static ref MD5_DUPE_IDENTIFIER: IdentifyDupeService = Box::<Md5DupeIdentifier>::default();
}

/// Returns a reference to the MD5 dupe identifier service singleton.
///
pub fn md5_dupe_identifier() -> &'static IdentifyDupeService {
    &MD5_DUPE_IDENTIFIER
}

/// MD5 dupe identifier.
///
/// This identifier uses the MD5 hash of the raw bytes to identify duplicates.
///
#[derive(Default, Debug)]
pub struct Md5DupeIdentifier {}

impl Identify for Md5DupeIdentifier {
    fn identify(&self, raw: &[u8]) -> String {
        format!("{:x}", md5::compute(raw))
    }
}

#[cfg(test)]
mod tests {
    use std::any::{Any, TypeId};

    use super::*;

    #[test]
    fn check_md5_dupe_identifier_singleton() {
        assert_eq!(
            md5_dupe_identifier().type_id(),
            TypeId::of::<IdentifyDupeService>()
        );
    }
}
