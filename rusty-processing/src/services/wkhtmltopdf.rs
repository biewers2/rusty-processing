use std::process::ExitStatus;

use lazy_static::lazy_static;
use tokio::io::{AsyncRead, AsyncWrite};
use crate::processing::ProcessOutput::Processed;

use crate::services::stream_command;

const PROGRAM: &str = "wkhtmltopdf";

const DEFAULT_ARGS: [&str; 15] = [
    "--quiet",
    "--encoding",
    "utf-8",
    "--disable-external-links",
    "--disable-internal-links",
    "--disable-forms",
    "--disable-local-file-access",
    "--disable-javascript",
    "--disable-toc-back-links",
    "--disable-plugins",
    "--proxy",
    "bogusproxy",
    "--proxy-hostname-lookup",
    "-",
    "-",
];

pub type WkhtmltopdfService = Box<Wkhtmltopdf>;

lazy_static! {
    static ref WKHTMLTOPDF: WkhtmltopdfService = Box::<Wkhtmltopdf>::default();
}

pub fn wkhtmltopdf() -> &'static WkhtmltopdfService {
    &WKHTMLTOPDF
}

#[derive(Default)]
pub struct Wkhtmltopdf {}

impl Wkhtmltopdf {
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
    async fn check_wkhtmltopdf_installed() -> anyhow::Result<()> {
        let mut proc = tokio::process::Command::new("which")
            .args(["wkhtmltopdf"])
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
    fn check_wkhtmltopdf_singleton() {
        assert_eq!(wkhtmltopdf().type_id(), TypeId::of::<Box<Wkhtmltopdf>>());
    }

    #[tokio::test]
    async fn test_wkhtmltopdf() {
        let input = b"hello world".to_vec();
        let mut output = vec![];
        let status = wkhtmltopdf().run(input.as_ref(), &mut output).await.unwrap();

        assert!(status.success());
        assert_ne!(output.len(), 0);
    }
}
