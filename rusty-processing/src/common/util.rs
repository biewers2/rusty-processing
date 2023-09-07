use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use mail_parser::ContentType;

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

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;

    #[test]
    fn test_write_file() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?.into_path();
        let path = temp_dir.join("test.txt");
        let contents = b"Hello, world!";
        write_file(&path, contents)?;

        assert_eq!(fs::read(&path)?, contents);
        Ok(())
    }

    #[test]
    fn test_mimetype_with_subtype() {
        let content_type = ContentType {
            c_type: Cow::from("text"),
            c_subtype: Some(Cow::from("plain")),
            attributes: None,
        };

        assert_eq!(mimetype(&content_type), "text/plain");
    }

    #[test]
    fn test_mimetype_without_subtype() {
        let content_type = ContentType {
            c_type: Cow::from("text"),
            c_subtype: None,
            attributes: None,
        };

        assert_eq!(mimetype(&content_type), "text");
    }
}