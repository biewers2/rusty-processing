use std::fs::File;
use std::io::{Seek, Write};
use std::path;
use anyhow::anyhow;
use bytesize::MB;
use tempfile::TempPath;

use tokio::io::AsyncReadExt;

pub struct ArchiveEntry {
    name: String,
    path: TempPath,
    id_chain: Vec<String>,
}

impl ArchiveEntry {
    pub fn new(name: String, path: TempPath, id_chain: Vec<String>) -> Self {
        Self { name, path, id_chain }
    }
}

pub struct ArchiveBuilder {
    zipper: zip::ZipWriter<File>,
}

impl ArchiveBuilder {
    pub fn new() -> anyhow::Result<Self> {
        let file = tempfile::tempfile()?;
        let zipper = zip::ZipWriter::new(file);

        Ok(Self { zipper })
    }

    pub async fn append(&mut self, entry: ArchiveEntry) -> anyhow::Result<()> {
        let path = self.archive_entry_path(&entry.id_chain, &entry.name);
        let path_parent = path.parent().ok_or(anyhow::anyhow!("No parent"))?;

        let path_string = path.to_string_lossy().to_string();
        let base_path_string = path_parent.to_string_lossy().to_string();

        self.zipper.add_directory(base_path_string, Default::default())?;
        self.zipper.start_file(path_string, Default::default())?;
        self.write_file(&entry.path).await?;
        Ok(())
    }

    pub fn build(&mut self) -> anyhow::Result<File> {
        let mut file = self.zipper.finish()?;
        file.rewind()?;
        Ok(file)
    }

    fn archive_entry_path(&self, embedded_dupe_chain: &[String], name: &str) -> path::PathBuf {
        let mut path = path::PathBuf::new();
        for dedupe_id in embedded_dupe_chain {
            path.push(dedupe_id);
        }
        path.push(name);
        path
    }

    async fn write_file(&mut self, path: &path::Path) -> anyhow::Result<()> {
        let mut file = tokio::fs::File::open(path).await?;

        let mut buf = Box::new([0; MB as usize]);
        loop {
            let bytes_read = file.read(buf.as_mut()).await?;
            if bytes_read == 0 {
                break;
            }
            if self.zipper.write(&buf[..bytes_read])? == 0 {
                return Err(anyhow!("ZIP writer unexpectedly closed"));
            }
        }
        Ok(())
    }
}