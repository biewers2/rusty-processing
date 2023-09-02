use mail_parser::ContentType;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::{fs, io};

use uuid::Uuid;

pub fn random_string() -> String {
    Uuid::new_v4().to_string()
}

pub fn write_file(path: &PathBuf, contents: &[u8]) -> anyhow::Result<()> {
    fs::create_dir_all(path.parent().unwrap())?;
    File::create(path)?.write_all(contents)?;
    Ok(())
}

pub fn mimetype(content_type: &ContentType) -> String {
    match (content_type.ctype(), content_type.subtype()) {
        (ctype, Some(subtype)) => format!("{}/{}", ctype, subtype),
        (ctype, None) => ctype.to_string(),
    }
}
