//!
//! Provides common services used for processing files.
//!
#![warn(missing_docs)]

use std::ffi::OsStr;
use std::fmt;
use std::fmt::Formatter;
use std::io::Cursor;
use std::ops::{Deref, DerefMut};
use std::process::{ExitStatus, Stdio};

use anyhow::{anyhow, Error};
use bytesize::MB;
use tokio::join;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

mod archive_builder;
mod config;
mod html_to_pdf;
mod pdf_to_image;
mod tika;
mod xdg_mime;

pub use archive_builder::*;
pub use config::*;
pub use html_to_pdf::*;
pub use pdf_to_image::*;
pub use tika::*;
pub use xdg_mime::*;

pub(crate) fn no_reader() -> Option<Cursor<Vec<u8>>> { None }

#[allow(dead_code)]
pub(crate) fn no_writer() -> Option<Vec<u8>> { None }

/// Error type for when a command execution fails.
///
#[derive(Debug, Clone)]
pub enum CommandError<E = Error> {
    /// When the command fails before exiting, such as if the child fails to spawn.
    ///
    PreExit(E),

    /// When the command fails after exiting, such as if the child exits with a non-zero status
    /// or the I/O streams encountered a problem during execution.
    ///
    PostExit(ExitStatus, E),
}

impl CommandError {
    /// Short-hand for creating a [`CommandError::PreExit`].
    ///
    /// Useful to pass in as a function handler to mapping functions (i.e. `map_err`).
    ///
    pub fn pre_exit(err: impl Into<Error>) -> Self {
        CommandError::PreExit(err.into())
    }

    /// Short-hand for creating a [`CommandError::PostExit`].
    ///
    /// Useful to pass in as a function handler to mapping functions (i.e. `map_err`).
    ///
    pub fn post_exit(status: ExitStatus, err: impl Into<Error>) -> Self {
        CommandError::PostExit(status, err.into())
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let (code, error) = match self {
            CommandError::PreExit(err) => ("".to_string(), err),
            CommandError::PostExit(status, err) => (
                status.code()
                    .map(|code| format!(" (code {})", code))
                    .unwrap_or("".to_string()),
                err
            )
        };

        write!(f, "{}{}", error, code)
    }
}

impl std::error::Error for CommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        let error = match self {
            CommandError::PreExit(err) => err,
            CommandError::PostExit(_, err) => err,
        };
        Some(error.as_ref())
    }
}

fn trim_to_string(value: &[u8]) -> String {
    String::from_utf8_lossy(value)
        .replace('\u{0}', "")
        .trim()
        .to_string()
}

async fn transfer<R, W>(reader: Option<R>, writer: Option<W>) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    if let (Some(mut reader), Some(mut writer)) = (reader, writer) {
        let mut buf = Box::new([0; MB as usize]);
        while reader.read(buf.deref_mut()).await? > 0 {
            if writer.write(buf.deref()).await? == 0 {
                return Err(anyhow!("writer closed unexpectedly"));
            }
        }
    }
    Ok(())
}

/// Run a command and return the exit status.
///
/// This function streams the input into stdin, stdout to the output, and stderr to the error asynchronously.
///
/// # Arguments
///
/// * `program` - The program to run.
/// * `arguments` - The arguments to pass to the program.
/// * `input` - An asynchronous read to stream into stdin.
/// * `output` - An asynchronous write to stream stdout into.
/// * `error` - An asynchronous write to stream stderr into.
///
/// # Returns
///
/// If the program exited successfully, the exit status is returned.
///
/// Otherwise, a [`CommandError`] is returned, and here are the possible implications:
/// 1. The function errored out before the command finished, so the exit status is [`None`] and the error will be populated
/// 2. The command finished, but an I/O error occurred while streaming, so the exit status and error will be populated
/// 2. The command finished, but the exit status was non-zero, so the exit status and error will be populated
///
/// For all errors that have an exit status, the `error` [`AsyncWrite`] passed to the function will have the `stderr` from the command.
///
pub(crate) async fn stream_command<R, W, E>(
    program: impl AsRef<str>,
    arguments: impl IntoIterator<Item=impl AsRef<OsStr>>,
    input: Option<R>,
    output: Option<W>,
    error: Option<E>,
) -> Result<ExitStatus, CommandError>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
        E: AsyncWrite + Unpin,
{
    let mut proc = tokio::process::Command::new(program.as_ref())
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(CommandError::pre_exit)?;

    let writing = transfer(input, proc.stdin.take());
    let reading = transfer(proc.stdout.take(), output);
    let erroring = transfer(proc.stderr.take(), error);

    // Don't `try_join!` to allow the error buffer to be written to completion
    let (writing_res, reading_res, erroring_res) = join!(writing, reading, erroring);
    let exit_status = proc.wait().await
        .map_err(CommandError::pre_exit)?;

    // Resolve the results after the process finishes to get the `ExitStatus`
    writing_res.and(reading_res).and(erroring_res)
        .map_err(|err| CommandError::post_exit(exit_status, err))?;

    if exit_status.success() {
        Ok(exit_status)
    } else {
        Err(CommandError::post_exit(exit_status, anyhow!("command failed with non-zero exit status")))
    }
}

#[cfg(test)]
mod test_utils {
    use std::process::Stdio;

    use tokio::io::AsyncReadExt;

    pub async fn assert_command_successful(command: impl Into<String>) -> anyhow::Result<(String, String)> {
        let command = command.into();
        let cmd_parts: Vec<&str> = command.split(' ').collect();
        let program = cmd_parts[0];
        let args = if cmd_parts.len() > 1 { &cmd_parts[1..] } else { &[] };

        let mut proc = tokio::process::Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut stdout = String::new();
        let mut stderr = String::new();
        proc.stdout.take().unwrap().read_to_string(&mut stdout).await?;
        proc.stderr.take().unwrap().read_to_string(&mut stderr).await?;
        let status = proc.wait().await?;

        assert!(status.success());
        Ok((stdout, stderr))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{CommandError, stream_command, trim_to_string};

    fn buffers(data: &[u8]) -> (Cursor<Vec<u8>>, Vec<u8>, Vec<u8>) {
        let input = Cursor::new(data.to_vec());
        let output = vec![];
        let error = vec![];
        (input, output, error)
    }

    #[tokio::test]
    async fn test_stream_command_succeeds() {
        let (mut input, mut output, mut error) = buffers(b"hello world");

        let result = stream_command(
            "cat",
            Vec::<&str>::new(),
            Some(&mut input),
            Some(&mut output),
            Some(&mut error),
        ).await;

        assert!(result.is_ok());
        let exit_status = result.unwrap();
        assert_eq!(exit_status.code(), Some(0));

        let output = trim_to_string(&output);
        assert_eq!(output, "hello world");
        assert!(error.is_empty());
    }

    #[tokio::test]
    async fn test_stream_command_fails_pre_exit() {
        let (mut input, mut output, mut error) = buffers(b"random input");

        let result = stream_command(
            "commandthatdoesntexist",
            vec!["random", "arguments"],
            Some(&mut input),
            Some(&mut output),
            Some(&mut error),
        ).await;

        assert!(result.is_err());
        let command_err = result.unwrap_err();
        if let CommandError::PreExit(err) = command_err {
            assert_eq!(err.to_string(), "No such file or directory (os error 2)");
        } else {
            panic!("expected pre-exit error");
        }
        assert!(output.is_empty());
        assert!(error.is_empty());
    }

    #[tokio::test]
    async fn test_stream_command_fails_post_exit_io() {
        let (mut input, mut output, mut error) = buffers(b"hello world");

        let result = stream_command(
            "ls",
            vec!["-abcde"],
            Some(&mut input),
            Some(&mut output),
            Some(&mut error),
        ).await;

        assert!(result.is_err());
        let command_err = result.unwrap_err();
        if let CommandError::PostExit(status, err) = command_err {
            assert_eq!(status.code(), Some(2));
            assert_eq!(err.to_string(), "Broken pipe (os error 32)");
        } else {
            panic!("expected post-exit error");
        }

        let error = trim_to_string(&error);
        assert!(output.is_empty());
        assert_eq!(error, "ls: invalid option -- 'e'\nTry 'ls --help' for more information.");
    }

    #[tokio::test]
    async fn test_stream_command_fails_post_exit_non_zero_status() {
        let (mut input, mut output, mut error) = buffers(b"");

        let result = stream_command(
            "bash",
            vec!["-c", "exit 13"],
            Some(&mut input),
            Some(&mut output),
            Some(&mut error),
        ).await;

        assert!(result.is_err());
        let command_err = result.unwrap_err();
        if let CommandError::PostExit(status, err) = command_err {
            assert_eq!(status.code(), Some(13));
            assert_eq!(err.to_string(), "command failed with non-zero exit status");
        } else {
            panic!("expected post-exit error");
        }

        assert!(output.is_empty());
        assert!(error.is_empty());
    }
}
