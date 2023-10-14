use std::process::ExitStatus;

use lazy_static::lazy_static;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{stream_command, trim_to_string};

const PROGRAM: &str = "gs";

const DEFAULT_ARGS: [&str; 8] = [
    "-q",             // No program metadata.json to stdout
    "-dNOPAUSE",      // Disable prompt/pause after end of each page
    "-dBATCH",        // Exit after operation exits
    "-dSAFER",        // Activate sandboxing; prevent I/O access outside specified files
    "-r300",          //
    "-sDEVICE=jpeg",  // Use JPEG image format
    "-sOutputFile=-", // Send metadata.json to stdout
    "-",              // Read input from stdin
];

/// The type of the singleton instance of the `PdfToImage` service.
///
pub type PdfToImageService = Box<PdfToImage>;

lazy_static! {
    static ref PDF_TO_IMAGE: PdfToImageService = Box::<PdfToImage>::default();
}

/// Returns the singleton instance of the `PdfToImage` service.
///
pub fn pdf_to_image() -> &'static PdfToImageService {
    &PDF_TO_IMAGE
}

/// The output of the `PdfToImage` service.
///
pub struct PdfToImageOutput {
    /// The exit status of the call to the `PdfToImage` CLI tool.
    ///
    pub exit_status: ExitStatus,

    /// The stderr of the call to the `PdfToImage` CLI tool.
    ///
    pub error: String,
}

/// The `PdfToImage` service.
///
#[derive(Default)]
pub struct PdfToImage {}

impl PdfToImage {
    /// Run the `PdfToImage` service.
    ///
    /// # Arguments
    ///
    /// * `input` - The input stream to read the PDF from.
    /// * `output` - The output stream to write the image to.
    ///
    /// # Returns
    ///
    /// * `Ok(PdfToImageOutput)` - If the `PdfToImage` CLI tool was run successfully.
    /// * `Err(_)` - If there was an error running the `PdfToImage` CLI tool.
    ///
    pub async fn run<R, W>(&self, mut input: R, mut output: W) -> anyhow::Result<PdfToImageOutput>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut error = vec![];
        let exit_status = stream_command(
            PROGRAM,
            &DEFAULT_ARGS,
            Some(&mut input),
            Some(&mut output),
            Some(&mut error),
        ).await?;

        Ok(PdfToImageOutput {
            exit_status,
            error: trim_to_string(&error),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::any::{Any, TypeId};
    
    
    use crate::test_utils::assert_command_successful;

    use super::*;

    #[tokio::test]
    async fn check_ghostscript_installed() {
        assert_command_successful("which gs").await.unwrap();
    }

    #[test]
    fn check_singleton() {
        assert_eq!(pdf_to_image().type_id(), TypeId::of::<Box<PdfToImage>>());
    }

    #[tokio::test]
    async fn test_pdf_to_img() {
        let input_path_str = "../resources/pdf/Espresso Machine Cleaning Guide.pdf";
        let input = tokio::fs::File::open(input_path_str).await.unwrap();
        let mut stdout = vec![];

        let output = pdf_to_image().run(input, &mut stdout).await.unwrap();

        assert!(output.exit_status.success());
        assert_eq!(output.error, "");
        assert_ne!(stdout.len(), 0);
    }
}
