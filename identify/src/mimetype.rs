use anyhow::anyhow;
use tokio::io::AsyncRead;
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
pub async fn identify_mimetype(content: impl AsyncRead + Send + Unpin) -> anyhow::Result<String> {
    let content = Box::pin(content);
    let output = tika().detect(content).await?;

    if output.status.success() {
        Ok(output.mimetype)
    } else {
        Err(anyhow!("failed to identify mimetype"))
    }
}

#[cfg(test)]
mod tests {
    
    
    use super::*;

    #[tokio::test]
    async fn test_detect_mimetype() -> anyhow::Result<()> {
        let file = tokio::fs::File::open("../resources/mbox/ubuntu-no-small.mbox").await?;
        let reader = tokio::io::BufReader::new(file);
        let mimetype = identify_mimetype(reader).await?;
        assert_eq!(mimetype, "application/mbox");
        Ok(())
    }
}