use std::process::ExitStatus;

use lazy_static::lazy_static;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::services::stream_command;

const PROGRAM: &str = "gs";

const DEFAULT_ARGS: [&str; 8] = [
    "-q",             // No program output to stdout
    "-dNOPAUSE",      // Disable prompt/pause after end of each page
    "-dBATCH",        // Exit after operation exits
    "-dSAFER",        // Activate sandboxing; prevent I/O access outside specified files
    "-r300",          //
    "-sDEVICE=jpeg",  // Use JPEG image format
    "-sOutputFile=-", // Send output to stdout
    "-",              // Read input from stdin
];

pub type PdfToImageService = Box<PdfToImage>;

lazy_static! {
    static ref PDF_TO_IMAGE: PdfToImageService = Box::<PdfToImage>::default();
}

pub fn pdf_to_image() -> &'static PdfToImageService {
    &PDF_TO_IMAGE
}

#[derive(Default)]
pub struct PdfToImage {}

impl PdfToImage {
    pub async fn run<R, W>(&self, mut input: R, mut output: W) -> anyhow::Result<ExitStatus>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut process = stream_command(PROGRAM, &DEFAULT_ARGS, &mut input, &mut output).await?;
        Ok(process.wait().await?)
    }
}

#[cfg(test)]
mod tests {
    use std::any::{Any, TypeId};
    use tokio::io::AsyncReadExt;

    use super::*;

    #[tokio::test]
    async fn check_ghostscript_installed() -> anyhow::Result<()> {
        let mut proc = tokio::process::Command::new("which")
            .args(["gs"])
            .stdout(Stdio::piped())
            .spawn()?;

        let mut output = String::new();
        proc.stdout.take().unwrap().read_to_string(&mut output).await?;
        let status = proc.wait().await?;

        assert!(status.success());
        assert_ne!(output, "".to_string());
        Ok(())
    }

    #[test]
    fn check_pdf_to_img_singleton() {
        assert_eq!(pdf_to_image().type_id(), TypeId::of::<Box<PdfToImage>>());
    }

    #[tokio::test]
    async fn test_pdf_to_img() {
        let input = b"hello world".to_vec();
        let mut output = vec![];
        let status = pdf_to_image().run(input.as_ref(), &mut output).await.unwrap();

        assert!(status.success());
        assert_ne!(output.len(), 0);
    }
}
