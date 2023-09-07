pub mod activities {
    pub mod create_workspace;
    pub mod destroy_workspace;
    pub mod download;
    pub(crate) mod multipart_uploader;
    pub mod process_rusty_file;
    pub mod upload;
}

pub(crate) mod util {
    pub mod parse_s3_uri;
    pub mod services;
}
