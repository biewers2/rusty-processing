use std::str::FromStr;

/// The type of output to produce from processing.
///
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ProcessType {
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

impl FromStr for ProcessType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(ProcessType::Text),
            "metadata" => Ok(ProcessType::Metadata),
            "pdf" => Ok(ProcessType::Pdf),
            _ => Err(format!("Can not convert {} to OutputType", s)),
        }
    }
}
