#![warn(missing_docs)]

pub mod common;
pub mod processing;
pub mod dupe_id;
pub(crate) mod message {
    pub mod rfc822;
}
pub(crate) mod application {
    pub mod mbox;
}
