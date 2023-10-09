use std::io::{Read, Seek};

use anyhow::anyhow;
use async_stream::stream;
use async_trait::async_trait;
use futures::{pin_mut, StreamExt};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use tempfile::TempPath;
use zip::ZipArchive;

use identify::deduplication::dedupe_checksum;
use identify::mimetype::identify_mimetype;
use streaming::{ByteStream, stream_to_read};

use crate::processing::{Process, ProcessContext, ProcessOutput};
use crate::temp_path;

enum NextArchiveEntry {
    Dir(String),
    File(ArchiveEntry),
}

struct ArchiveEntry {
    name: String,
    path: TempPath,
    dedupe_checksum: String,
    mimetype: String,
}

async fn next_archive_entry<R>(archive: &mut ZipArchive<R>, index: usize) -> anyhow::Result<NextArchiveEntry>
where R: Read + Seek
{
    // Create an inner scope because `ZipFile` is not `Send` and must be dropped before `await`ing
    let (name, path) = {
        let mut zipfile = archive.by_index(index)?;

        let name = zipfile.enclosed_name()
            .and_then(|name| name.file_name())
            .map(|name| name.to_string_lossy().to_string())
            .ok_or(anyhow!("failed to get name for zip entry"))?;

        if zipfile.is_dir() {
            return Ok(NextArchiveEntry::Dir(name));
        }

        let emb_path = spool_read(&mut zipfile)?;
        (name, emb_path)
    };

    let emb_file = tokio::fs::File::open(&path).await?;
    let mimetype = identify_mimetype(emb_file).await?;

    let emb_file = tokio::fs::File::open(&path).await?;
    let checksum = dedupe_checksum(emb_file, &mimetype).await?;

    Ok(NextArchiveEntry::File(ArchiveEntry { name, path, dedupe_checksum: checksum, mimetype }))
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
        info!("Spooling zip file");
        let mut read = stream_to_read(stream).await?;
        let archive_path = spool_read(&mut read)?;

        info!("Opening zip file");
        let archive_file = std::fs::File::open(&archive_path)?;
        let mut archive = ZipArchive::new(archive_file)?;

        info!("Streaming zip file entries");
        let output_stream = stream! {
            for i in 0..archive.len() {
                yield next_archive_entry(&mut archive, i).await;
            }
        };

        pin_mut!(output_stream);
        while let Some(result) = output_stream.next().await {
            match result {
                Ok(NextArchiveEntry::Dir(name)) => {
                    debug!("Discovered ZIP directory {}", name);
                },

                Ok(NextArchiveEntry::File(entry)) => {
                    let ArchiveEntry { name, path, dedupe_checksum, mimetype } = entry;
                    let output = ProcessOutput::embedded(&ctx, name, path, mimetype, dedupe_checksum);
                    ctx.add_output(Ok(output)).await?;
                },

                Err(e) => {
                    warn!("Failed to read ZIP entry: {}", e);
                }
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "zip"
    }
}
