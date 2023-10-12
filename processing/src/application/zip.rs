use std::io::{Read, Seek};
use std::path::PathBuf;

use anyhow::anyhow;
use async_stream::stream;
use async_trait::async_trait;
use futures::{pin_mut, StreamExt};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use tempfile::{NamedTempFile, TempPath};
use zip::ZipArchive;

use identify::deduplication::dedupe_checksum_from_path;
use identify::mimetype::identify_mimetype;

use crate::processing::{Process, ProcessContext, ProcessOutput};

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

#[derive(Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct ZipProcessor;

#[async_trait]
impl Process for ZipProcessor {
    async fn process(&self, ctx: ProcessContext, path: PathBuf) -> anyhow::Result<()> {
        info!("Opening zip file");
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let mut archive = ZipArchive::new(reader)?;

        info!("Streaming zip file entries");
        let output_stream = stream! {
            for i in 0..archive.len() {
                yield next_archive_entry(&mut archive, i).await;
            }
        };

        pin_mut!(output_stream);
        while let Some(result) = output_stream.next().await {
            match result {
                Ok(NextArchiveEntry::File(entry)) => {
                    info!("Discovered ZIP file {}", entry.name);
                    let ArchiveEntry { name, path, dedupe_checksum, mimetype } = entry;
                    let output = ProcessOutput::embedded(&ctx, name, path, mimetype, dedupe_checksum);
                    ctx.add_output(Ok(output)).await?;
                },
                Ok(NextArchiveEntry::Dir(name)) => debug!("Discovered ZIP directory {}", name),
                Err(e) => warn!("Failed to read ZIP entry: {}", e),
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "zip"
    }
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

    let mimetype = identify_mimetype(&path).await?.unwrap_or("application/octet-stream".to_string());
    let checksum = dedupe_checksum_from_path(&path, &mimetype).await?;

    Ok(NextArchiveEntry::File(ArchiveEntry { name, path, dedupe_checksum: checksum, mimetype }))
}

/// Write contents to a temporary file and return the temporary path.
///
fn spool_read(mut reader: impl Read) -> anyhow::Result<TempPath> {
    let mut file = NamedTempFile::new()?;
    std::io::copy(&mut reader, &mut file)?;
    Ok(file.into_temp_path())
}