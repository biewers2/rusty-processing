use std::path::Path;

use file_format::FileFormat;
use log::info;

use services::{tika, xdg_mime};

/// Identifies the mimetype of a file.
///
/// # Arguments
///
/// * `content` - The file contents to identify the mimetype for.
///
/// # Returns
///
/// The mimetype of the file.
///
pub async fn identify_mimetype(path: impl AsRef<Path>) -> anyhow::Result<Option<String>> {
    if let Some(mimetype) = identify_using_xdg_mime(&path).await? {
        info!("Identified mimetype as '{}' using 'xdg-mime'", mimetype);
        return Ok(Some(mimetype));
    }

    if let Some(mimetype) = identify_using_tika(&path).await? {
        info!("Identified mimetype as '{}' using Tika", mimetype);
        return Ok(Some(mimetype));
    }

    if let Some(mimetype) = identify_using_file_format(&path).await? {
        info!("Identified mimetype as '{}' using file format", mimetype);
        return Ok(Some(mimetype));
    }

    Ok(None)
}

async fn identify_using_xdg_mime(path: impl AsRef<Path>) -> anyhow::Result<Option<String>> {
    let mimetype = xdg_mime().query_filetype(path).await?;
    Ok((mimetype != "application/octet-stream" && mimetype != "text/plain").then_some(mimetype))
}

async fn identify_using_tika(path: impl AsRef<Path>) -> anyhow::Result<Option<String>> {
    let mimetype = tika().detect(path).await?;
    Ok((mimetype != "application/octet-stream").then_some(mimetype))
}

async fn identify_using_file_format(path: impl AsRef<Path>) -> anyhow::Result<Option<String>> {
    let mimetype = FileFormat::from_file(path)?.media_type().to_string();
    Ok((mimetype != "application/octet-stream").then_some(mimetype))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_mimetype() -> anyhow::Result<()> {
        let path = "../resources/mbox/ubuntu-no-small.mbox";

        let mimetype = identify_mimetype(path).await?;

        assert!(mimetype.is_some());
        assert_eq!(mimetype.unwrap(), "application/mbox");
        Ok(())
    }
}