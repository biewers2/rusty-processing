use std::fs::File;
use std::io::Read;
use std::path;

pub fn read_contents(path: &str) -> anyhow::Result<Vec<u8>> {
    let mut content = vec![];
    File::open(path::PathBuf::from(path))?.read_to_end(&mut content)?;
    Ok(content)
}
