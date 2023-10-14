use std::path::Path;

use anyhow::anyhow;
use lazy_static::lazy_static;

use crate::{CommandError, no_reader, stream_command, trim_to_string};

/// The type of the singleton instance of the `XdgMime` service.
///
pub type XdgMimeService = Box<XdgMime>;

lazy_static! {
    static ref XDG_MIME: XdgMimeService = Box::<XdgMime>::default();
}

/// Returns the singleton instance of the `xdg-mime` service.
pub fn xdg_mime() -> &'static XdgMimeService {
    &XDG_MIME
}

/// The `xdg-mime` service.
///
#[derive(Default)]
pub struct XdgMime;

impl XdgMime {
    /// Run the `xdg-mime` service to identify the mimetype of a file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file to identify.
    ///
    /// # Returns
    ///
    /// The mimetype of the file.
    ///
    pub async fn query_filetype(&self, path: impl AsRef<Path>) -> anyhow::Result<String> {
        let path_str = path.as_ref().to_str().ok_or(anyhow!("failed to convert path to string"))?;

        let mut output = vec![];
        let mut error = vec![];
        let result = stream_command(
            "xdg-mime",
            &["query", "filetype", path_str],
            no_reader(),
            Some(&mut output),
            Some(&mut error),
        ).await;

        match result {
            Ok(_) => Ok(trim_to_string(&output)),
            Err(CommandError::PreExit(err)) => Err(err),
            Err(CommandError::PostExit(status, err)) => {
                let code = status.code()
                    .map(|c| c.to_string())
                    .unwrap_or("?".to_string());
                Err(anyhow!("'xdg-mime' failed to detect mimetype: {} (code {}): {}", err, code, trim_to_string(&error)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::{Any, TypeId};

    use super::*;

    #[test]
    fn check_singleton() {
        assert_eq!(xdg_mime().type_id(), TypeId::of::<Box<XdgMime>>());
    }

    #[tokio::test]
    async fn test_query_filetype() {
        let cases = vec![
            ("../resources/mbox/ubuntu-no-small.mbox", "application/mbox"),
            ("../resources/rfc822/headers-small.eml", "message/rfc822"),
            ("../resources/jpg/PA280041.JPG", "image/jpeg"),
        ];

        for (path, expected) in cases {
            let result = xdg_mime().query_filetype(path).await;

            assert!(result.is_ok(), "failed to query filetype for '{}'", path);
            assert_eq!(result.unwrap(), expected);
        }
    }

    #[tokio::test]
    async fn test_query_filetype_missing_path() {
        let expected_err = "\
'xdg-mime' failed to detect mimetype: \
command failed with non-zero exit status (code 2): \
xdg-mime: file 'path-does-not-exist' does not exist";
        let path = "path-does-not-exist";

        let result = xdg_mime().query_filetype(path).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), expected_err);
    }
}