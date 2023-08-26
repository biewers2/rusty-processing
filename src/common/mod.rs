/// Defines the interface used for all error reporting and handling.
///
pub mod error;

/// Defines the possible outputs for processed files.
///
pub mod output_type;

/// Service for converting HTML to PDF.
///
pub(crate) mod wkhtmltopdf;

/// Service for creating a "workspace" for processing files.
///
/// A workspace is a location inside a specified directory that is used to store the file output from processing.
///
pub(crate) mod workspace;
