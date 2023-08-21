use std::io;

pub(super) fn call() -> io::Result<String> {
    Ok("metadata".to_string())
}