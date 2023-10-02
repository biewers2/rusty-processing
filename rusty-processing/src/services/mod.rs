/// Service for converting HTML to PDF.
///
mod wkhtmltopdf;
mod pdf_to_image;

use std::ffi::OsStr;
use std::process::Stdio;
pub(crate) use wkhtmltopdf::*;


use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::try_join;

async fn stream_command<R, W, S>(
    _program: impl AsRef<str>,
    arguments: impl IntoIterator<Item=S>,
    mut input: R,
    mut output: W,
) -> anyhow::Result<tokio::process::Child>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    S: AsRef<OsStr>,
{
    let mut proc = tokio::process::Command::new("wkhtmltopdf")
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut stdin = proc.stdin.take();
    let mut stdout = proc.stdout.take();

    let writing = async move {
        if let Some(mut stdin) = stdin.take() {
            let mut buf = [0; 10000];
            while input.read(&mut buf).await? > 0 {
                stdin.write(&buf).await?;
            }
        }
        anyhow::Ok(())
    };

    let reading = async move {
        if let Some(mut stdout) = stdout.take() {
            let mut buf = [0; 10000];
            while stdout.read(&mut buf).await? > 0 {
                output.write(&buf).await?;
            }
        }
        anyhow::Ok(())
    };

    try_join!(writing, reading)?;
    Ok(proc)
}