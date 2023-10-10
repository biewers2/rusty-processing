use std::io::Cursor;
use std::ops::{Deref, DerefMut};
use std::path::Path;

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
/// * `path` - Path to the file to calculate the checksum for.
/// * `mimetype` - The mimetype of the file.
///
/// # Returns
///
/// The checksum as a string.
///
pub async fn dedupe_checksum_from_path(path: impl AsRef<Path>, mimetype: impl AsRef<str>) -> anyhow::Result<String> {
    let checksum = match mimetype.as_ref() {
        "message/rfc822" => dedupe_message_from_path(path).await,
        _ => dedupe_md5_from_path(path).await,
    }?;
    Ok(checksum)
}

/// Calculates a checksum that represents a unique identification of a file.
///
/// This checksum can be used to identify duplicate files.
///
/// # Arguments
///
/// * `content` - Content to calculate the checksum for.
/// * `mimetype` - The mimetype of the file.
///
/// # Returns
///
/// The checksum as a string.
///
pub async fn dedupe_checksum(content: &mut (impl AsyncRead + Unpin), mimetype: impl AsRef<str>) -> anyhow::Result<String> {
    let checksum = match mimetype.as_ref() {
        "message/rfc822" => dedupe_message(content).await,
        _ => dedupe_md5(content).await,
    }?;
    Ok(checksum)
}

async fn dedupe_md5_from_path(path: impl AsRef<Path>) -> anyhow::Result<String> {
    let mut content = tokio::fs::File::open(path).await?;
    dedupe_md5(&mut content).await
}

async fn dedupe_md5(content: &mut (impl AsyncRead + Unpin)) -> anyhow::Result<String> {
    let mut ctx = md5::Context::new();
    let mut buf = Box::new([0; MB as usize]);
    while content.read(buf.deref_mut()).await? > 0 {
        ctx.consume(buf.deref());
    }
    Ok(format!("{:x}", ctx.compute()))
}

async fn dedupe_message_from_path(path: impl AsRef<Path>) -> anyhow::Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    dedupe_message(&mut file).await
}

async fn dedupe_message(content: &mut (impl AsyncRead + Unpin)) -> anyhow::Result<String> {
    let mut buf = vec![];
    content.read_to_end(&mut buf).await?;

    let message = MessageParser::default().parse(&buf);
    let raw_id = message
        .as_ref()
        .and_then(|msg| msg.message_id())
        .map(|id| id.as_bytes().to_vec());

    let mut content = raw_id
        .map(|raw_id| Box::new(Cursor::new(raw_id)))
        .unwrap_or(Box::new(Cursor::new(buf)));

    dedupe_md5(&mut content).await
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use crate::deduplication::dedupe_checksum;

    #[tokio::test]
    async fn test_dedupe_checksum_message_no_data() {
        let mut content = Cursor::new(b"".to_vec());

        let checksum = dedupe_checksum(&mut content, "message/rfc822").await.unwrap();

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
        let mut content = Cursor::new(content.to_vec());

        let checksum = dedupe_checksum(&mut content, "message/rfc822").await.unwrap();

        assert_eq!(checksum, "48746efe196a27e395f613b9c0773b8b");
    }

    #[tokio::test]
    async fn test_dedupe_checksum_md5_no_data() {
        let mut content = Cursor::new(b"".to_vec());

        let checksum = dedupe_checksum(&mut content, "application/octet-stream").await.unwrap();

        assert_eq!(checksum, "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[tokio::test]
    async fn test_dedupe_checksum_md5() {
        let mut content = Cursor::new(b"Hello, world!".to_vec());

        let checksum = dedupe_checksum(&mut content, "application/octet-stream").await.unwrap();

        assert_eq!(checksum, "bccf69bd7101c797b298c8b5329b965f");
    }
}
