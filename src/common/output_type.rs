use std::str::FromStr;

/// The type of output to produce from processing.
///
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum OutputType {
    /// Extracted text of a file.
    ///
    Text,

    /// Metadata of a file.
    ///
    Metadata,

    /// A rendered version of a file as a PDF.
    ///
    Pdf,
}

impl FromStr for OutputType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputType::Text),
            "metadata" => Ok(OutputType::Metadata),
            "pdf" => Ok(OutputType::Pdf),
            _ => Err(format!("Can not convert {} to OutputType", s)),
        }
    }
}
