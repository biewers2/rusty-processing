use std::fs::File;
use std::io::Seek;
use std::path::Path;

pub struct ZipBuilder {
    zipper: zip::ZipWriter<File>,
}

impl ZipBuilder {
    pub fn new() -> anyhow::Result<Self> {
        let file = tempfile::tempfile()?;
        let zipper = zip::ZipWriter::new(file);

        Ok(Self { zipper })
    }

    pub fn add_new(&mut self, file_path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = file_path.as_ref();
        let path_parent = path.parent().ok_or(anyhow::anyhow!("No parent"))?;

        let path_string = path.to_string_lossy().to_string();
        let base_path_string = path_parent.to_string_lossy().to_string();

        self.zipper.add_directory(base_path_string, Default::default())?;
        self.zipper.start_file(path_string, Default::default())?;
        Ok(())
    }

    pub fn build(&mut self) -> anyhow::Result<File> {
        let mut file = self.zipper.finish()?;
        file.rewind()?;
        Ok(file)
    }
}