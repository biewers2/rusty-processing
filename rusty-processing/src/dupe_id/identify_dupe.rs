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

/// Returns the [`IdentifyDupeService`] singleton for the given mimetype.
///
pub fn identifier(mimetype: &String) -> &'static IdentifyDupeService {
    match mimetype.as_str() {
        "message/rfc822" => message_dupe_identifier(),
        _ => md5_dupe_identifier(),
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

        let dupe_id = identifier(&"message/rfc822".to_string()).identify(content.as_bytes());

        assert_eq!(dupe_id, "a067755b964bda62fc2d1a9557994852");
        Ok(())
    }

    #[test]
    fn test_identifier_identifies_anything() -> anyhow::Result<()> {
        let content = b"Hello, world!";
        let dupe_id = identifier(&"application/octet-stream".to_string()).identify(content);

        assert_eq!(dupe_id, "6cd3556deb0da54bca060b4c39479839");
        Ok(())
    }
}
