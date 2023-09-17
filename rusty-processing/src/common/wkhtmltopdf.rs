use lazy_static::lazy_static;
use std::io::{Read, Write};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;

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
    pub fn run(&self, input: &[u8], output: &mut Vec<u8>) -> anyhow::Result<ExitStatus> {
        let mut proc = Command::new("wkhtmltopdf")
            .args(DEFAULT_ARGS)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut stdin = proc.stdin.take();
        let mut stdout = proc.stdout.take();

        thread::scope(move |_| {
            if let Some(mut stdin) = stdin.take() {
                stdin.write_all(input).unwrap();
            }
        });

        if let Some(mut stdout) = stdout.take() {
            stdout.read_to_end(output)?;
        }

        Ok(proc.wait()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::{Any, TypeId};

    #[test]
    fn check_wkhtmltopdf_installed() -> anyhow::Result<()> {
        let mut proc = Command::new("which")
            .args(&["wkhtmltopdf"])
            .stdout(Stdio::piped())
            .spawn()?;

        let mut output = String::new();
        proc.stdout.take().unwrap().read_to_string(&mut output)?;
        let status = proc.wait()?;

        assert!(status.success());
        assert_ne!(output, "".to_string());
        Ok(())
    }

    #[test]
    fn check_wkhtmltopdf_singleton() {
        assert_eq!(wkhtmltopdf().type_id(), TypeId::of::<Box<Wkhtmltopdf>>());
    }

    #[test]
    fn test_wkhtmltopdf() {
        let mut output = vec![];
        let status = wkhtmltopdf().run(b"hello world", &mut output).unwrap();

        assert!(status.success());
        assert_ne!(output.len(), 0);
    }
}
