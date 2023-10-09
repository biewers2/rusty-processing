use file_format::FileFormat;
use log::info;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use services::tika;

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
pub async fn identify_mimetype<R>(mut content: R) -> anyhow::Result<Option<String>>
    where R: AsyncRead + AsyncSeek + Send + Sync + Unpin + 'static
{
    if let Some(mimetype) = identify_using_file_format(&mut content).await? {
        info!("Identified mimetype as '{}' using file format", mimetype);
        return Ok(Some(mimetype));
    }

    content.rewind().await?;
    if let Some(mimetype) = identify_using_tika(content).await? {
        info!("Identified mimetype as '{}' using Tika", mimetype);
        return Ok(Some(mimetype));
    }

    Ok(None)
}

async fn identify_using_file_format<R>(content: &mut R) -> anyhow::Result<Option<String>>
    where R: AsyncRead + Send + Sync + Unpin + 'static
{
    let mut buf = Vec::new();
    content.read_to_end(&mut buf).await?;

    let format = FileFormat::from_bytes(&buf);

    let mimetype = format.media_type().to_string();
    Ok((mimetype != "application/octet-stream").then_some(mimetype))
}

async fn identify_using_tika<R>(content: R) -> anyhow::Result<Option<String>>
    where R: AsyncRead + Send + Sync + Unpin + 'static
{
    let mimetype = tika().detect(content).await?;
    Ok((mimetype != "application/octet-stream").then_some(mimetype))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detect_mimetype() -> anyhow::Result<()> {
        let file = tokio::fs::File::open("../resources/mbox/ubuntu-no-small.mbox").await?;
        let reader = tokio::io::BufReader::new(file);
        let mimetype = identify_mimetype(reader).await?;
        assert!(mimetype.is_some());
        assert_eq!(mimetype.unwrap(), "application/mbox");
        Ok(())
    }
}