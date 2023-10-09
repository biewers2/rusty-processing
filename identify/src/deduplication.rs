use std::io::Cursor;
use std::ops::{Deref, DerefMut};

use bytesize::MB;
use log::info;
use mail_parser::MessageParser;
use tokio::io::{AsyncRead, AsyncReadExt};

/// Calculates a checksum that represents a unique identification of a file.
///
/// This checksum can be used to identify duplicate files.
///
/// # Arguments
///
/// * `content` - The file contents to calculate the checksum for; this must be a seekable stream,
/// so contents can be read twice, first for identifying the mimetype, second for calculating the
/// checksum.
///
/// # Returns
///
/// A [`Deduplication`] struct containing the checksum and the mimetype of the file.
///
pub async fn dedupe_checksum<R>(content: R, mimetype: impl AsRef<str>) -> anyhow::Result<String>
where R: AsyncRead + Send + Sync + Unpin
{
    let content = Box::new(content);
    let checksum = match mimetype.as_ref() {
        "message/rfc822" => dedupe_message(content).await,
        _ => dedupe_md5(content).await,
    }?;
    info!("Calculated dedupe checksum: {}", checksum);
    Ok(checksum)
}

/// MD5 dupe identifier.
///
/// This identifier uses the MD5 hash of the raw bytes to identify duplicates.
///
async fn dedupe_md5<'a, R>(mut content: R) -> anyhow::Result<String>
where R: AsyncRead + Send + Unpin + 'a
{
    let mut ctx = md5::Context::new();
    let mut buf = Box::new([0; MB as usize]);
    while content.read(buf.deref_mut()).await? > 0 {
        ctx.consume(buf.deref());
    }
    Ok(format!("{:x}", ctx.compute()))
}

/// Identifies a message by its message ID, or if it doesn't have one, by a
/// randomly generated UUID.
///
async fn dedupe_message<'a, R>(mut content: R) -> anyhow::Result<String>
where R: AsyncRead + Send + Unpin + 'a
{
    let mut buf = Vec::new();
    content.read_to_end(&mut buf).await?;

    let message = MessageParser::default().parse(&buf);
    let raw_id = message
        .as_ref()
        .and_then(|msg| msg.message_id())
        .map(|id| id.as_bytes().to_vec());

    let content = match raw_id {
        Some(raw_id) => Box::new(Cursor::new(raw_id)),
        None => Box::new(Cursor::new(buf)),
    };
    dedupe_md5(content).await
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use crate::deduplication::dedupe_checksum;

    #[tokio::test]
    async fn test_dedupe_checksum_message_no_data() {
        let content = Cursor::new(b"".to_vec());

        let checksum = dedupe_checksum(content, "message/rfc822").await.unwrap();

        assert_eq!(checksum, "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[tokio::test]
    async fn test_dedupe_checksum_message() {
        let content = b"\
Message-ID: <1449186.1075855697095.JavaMail.evans@thyme>
Date: Wed, 21 Feb 2001 07:58:00 -0800 (PST)
From: phillip.allen@enron.com
To: cbpres@austin.rr.com
Subject: Re: Weekly Status Meeting
Mime-Version: 1.0
Content-Type: text/plain; charset=us-ascii
Content-Transfer-Encoding: 7bit

Tomorrow is fine.  Talk to you then.

Phillip";
        let content = Cursor::new(content.to_vec());

        let checksum = dedupe_checksum(content, "message/rfc822").await.unwrap();

        assert_eq!(checksum, "48746efe196a27e395f613b9c0773b8b");
    }

    #[tokio::test]
    async fn test_dedupe_checksum_md5_no_data() {
        let content = Cursor::new(b"".to_vec());

        let checksum = dedupe_checksum(content, "application/octet-stream").await.unwrap();

        assert_eq!(checksum, "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[tokio::test]
    async fn test_dedupe_checksum_md5() {
        let content = Cursor::new(b"Hello, world!".to_vec());

        let checksum = dedupe_checksum(content, "application/octet-stream").await.unwrap();

        assert_eq!(checksum, "bccf69bd7101c797b298c8b5329b965f");
    }
}
