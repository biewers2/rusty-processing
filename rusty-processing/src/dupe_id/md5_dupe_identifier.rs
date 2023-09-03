use lazy_static::lazy_static;

use crate::dupe_id::identify_dupe::{IdentifyDupe, IdentifyDupeService};

lazy_static! {
    static ref MD5_DUPE_IDENTIFIER: IdentifyDupeService = Box::<Md5DupeIdentifier>::default();
}

/// Returns a reference to the MD5 dupe identifier service singleton.
///
pub fn md5_dupe_identifier() -> &'static IdentifyDupeService {
    &MD5_DUPE_IDENTIFIER
}

#[derive(Default)]
struct Md5DupeIdentifier {}

impl IdentifyDupe for Md5DupeIdentifier {
    fn identify(&self, raw: &[u8]) -> String {
        format!("{:x}", md5::compute(raw))
    }
}