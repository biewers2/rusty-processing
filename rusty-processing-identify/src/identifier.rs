use crate::identify::IdentifyDedupeService;
use crate::md5_dedupe_identifier;
use crate::message_dedupe_identifier;

/// Returns the [`IdentifyDedupeService`] singleton for the given mimetype.
///
pub fn identifier(mimetype: impl AsRef<str>) -> &'static IdentifyDedupeService {
    match mimetype.as_ref() {
        "message/rfc822" => message_dedupe_identifier(),
        _ => md5_dedupe_identifier(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Read;
    use std::path;

    use super::*;

    #[test]
    fn test_identifier_identifies_messages() -> anyhow::Result<()> {
        let path = path::PathBuf::from("resources/rfc822/allen-p-all-docs-503.eml");
        let mut content = String::new();
        File::open(path)?.read_to_string(&mut content)?;

        let dupe_id = identifier("message/rfc822").identify(content.as_bytes());

        assert_eq!(dupe_id, "a067755b964bda62fc2d1a9557994852");
        Ok(())
    }

    #[test]
    fn test_identifier_identifies_anything() -> anyhow::Result<()> {
        let content = b"Hello, world!";
        let dupe_id = identifier("application/octet-stream").identify(content);

        assert_eq!(dupe_id, "6cd3556deb0da54bca060b4c39479839");
        Ok(())
    }
}
