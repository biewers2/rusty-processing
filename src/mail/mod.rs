mod text;
mod metadata;
mod pdf;

use std::io;

pub fn text() -> io::Result<String> {
    text::call()
}

pub fn metadata() -> io::Result<String> {
    metadata::call()
}

pub fn pdf() -> io::Result<String> {
    pdf::call()
}