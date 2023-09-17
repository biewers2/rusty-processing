/// Service for converting HTML to PDF.
///
pub(crate) mod wkhtmltopdf;

/// Service for creating a "workspace" for processing files.
///
/// A workspace is a location inside a specified directory that is used to store the file output from processing.
///
pub(crate) mod workspace;

pub(crate) mod util;
