#![warn(missing_docs)]

pub mod activities {
    pub mod process_rusty_file;
}

pub(crate) mod io {
    pub mod download;
    pub mod upload;
    pub mod multipart_uploader;
}

pub(crate) mod util;
pub(crate) mod services;
