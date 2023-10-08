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
pub async fn identify_mimetype(content: impl AsyncRead + Send + Sync + Unpin + 'static) -> anyhow::Result<String> {
    let content = Box::pin(content);
    tika().detect(content).await
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