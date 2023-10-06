use std::io;
use std::io::Read;

use anyhow::anyhow;
use async_stream::stream;
use async_trait::async_trait;
use futures::{pin_mut, StreamExt};
use serde::{Deserialize, Serialize};
use tempfile::TempPath;
use zip::ZipArchive;

use identify::deduplication::{dedupe_checksum, Deduplication};

use crate::io::{ByteStream, stream_to_read, temp_path};
use crate::processing::{Process, ProcessContext, ProcessOutput};

struct ArchiveEntry {
    name: String,
    path: TempPath,
    dedupe: Deduplication,
}

async fn next_archive_entry<R>(archive: &mut ZipArchive<R>, index: usize) -> anyhow::Result<ArchiveEntry>
where R: Read + io::Seek
{
    // Create an inner scope because `ZipFile` is not `Send` and must be dropped before `await`ing
    let (name, path) = {
        let mut zipfile = archive.by_index(index)?;
        let name = zipfile.enclosed_name()
            .and_then(|name| name.file_name())
            .map(|name| name.to_string_lossy().to_string())
            .ok_or(anyhow!("failed to get name for zip entry"))?;
        let emb_path = spool_read(&mut zipfile)?;
        (name, emb_path)
    };

    let mut emb_file = tokio::fs::File::open(&path).await?;
    let dedupe = dedupe_checksum(&mut emb_file).await?;

    Ok(ArchiveEntry {
        name,
        path,
        dedupe,
    })
}

/// Write contents to a temporary file and return the temporary path.
///
fn spool_read(mut reader: impl Read) -> anyhow::Result<TempPath> {
    let path = temp_path()?;
    let file = std::fs::File::create(&path)?;
    let mut writer = std::io::BufWriter::new(file);
    std::io::copy(&mut reader, &mut writer)?;
    Ok(path)
}

#[derive(Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct ZipProcessor;

#[async_trait]
impl Process for ZipProcessor {
    async fn process(&self, ctx: ProcessContext, stream: ByteStream) -> anyhow::Result<()> {
        let mut read = stream_to_read(stream).await?;
        let archive_path = spool_read(&mut read)?;

        let reader = std::fs::File::open(&archive_path)?;
        let mut archive = zip::ZipArchive::new(reader)?;

        let output_stream = stream! {
            for i in 0..archive.len() {
                yield next_archive_entry(&mut archive, i).await;
            }
        };

        pin_mut!(output_stream);
        while let Some(result) = output_stream.next().await {
            let ArchiveEntry { name, path, dedupe } = result?;
            let output = ProcessOutput::embedded(
                &ctx,
                name,
                path,
                dedupe.mimetype,
                dedupe.checksum,
            );
            ctx.add_output(Ok(output)).await?;
        }

        Ok(())
    }
}