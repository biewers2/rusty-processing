use std::process::ExitStatus;
use anyhow::anyhow;

use lazy_static::lazy_static;
use reqwest::Body;
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::{stream_command, trim_to_string};

const PROGRAM: &str = "java";
const TIKA_JAR: &str = "../bin/tika-app-2.9.0.jar";
const DEFAULT_ARGS: [&str; 2] = [
    "-jar",
    TIKA_JAR,
];

pub type TikaService = Box<Tika>;

lazy_static! {
    static ref TIKA: TikaService = Box::<Tika>::default();
}

pub fn tika() -> &'static TikaService {
    &TIKA
}

pub type TikaTextOutput = TikaOutput;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TikaMetadataOutput {
    pub status: ExitStatus,
    pub json: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TikaOutput {
    pub status: ExitStatus,
    pub error: String,
}

pub struct Tika {
    http_client: reqwest::Client,
}

impl Default for Tika {
    fn default() -> Self {
        Self { http_client: reqwest::Client::new() }
    }
}

impl Tika {
    pub async fn text<R, W>(&self, input: R, output: W) -> anyhow::Result<TikaTextOutput>
        where
            R: AsyncRead + Unpin,
            W: AsyncWrite + Unpin,
    {
        let TikaOutput { status, error } = self.run(vec!["--text"], input, output).await?;
        Ok(TikaTextOutput { status, error })
    }

    pub async fn metadata<R>(&self, input: R) -> anyhow::Result<TikaMetadataOutput>
        where R: AsyncRead + Unpin
    {
        let mut stdout = vec![];
        let TikaOutput { status, error } = self.run(vec!["--metadata", "--json"], input, &mut stdout).await?;
        Ok(TikaMetadataOutput {
            status,
            json: trim_to_string(&stdout),
            error,
        })
    }

    /// Detects the mimetype of the input file.
    ///
    /// # Arguments
    ///
    /// * `input` - The content representing the file to detect the mimetype of.
    ///
    /// # Returns
    ///
    /// The mimetype of the input file.
    ///
    pub async fn detect<R>(&self, input: R) -> anyhow::Result<String>
        where R: AsyncRead + Send + Sync + Unpin + 'static
    {
        let stream = FramedRead::new(input, BytesCodec::new());
        let body = Body::wrap_stream(stream);

        let response = self.http_client
            .put("http://localhost:9998/meta/Content-Type")
            .header("Accept", "application/json")
            .body(body)
            .send().await?;
        let mimetype = self.parse_detect_response(response).await?;
        Ok(mimetype)
    }

    pub async fn run<R, W>(&self, mut args: Vec<&str>, mut input: R, mut output: W) -> anyhow::Result<TikaOutput>
        where
            R: AsyncRead + Unpin,
            W: AsyncWrite + Unpin,
    {
        let mut arguments = DEFAULT_ARGS.to_vec();
        arguments.append(&mut args);

        let mut stderr = vec![];
        let result = stream_command(PROGRAM, arguments, &mut input, &mut output, &mut stderr).await;

        match result {
            Ok(status) => Ok(TikaOutput {
                status,
                error: trim_to_string(&stderr),
            }),
            Err(cmd_err) => {
                let error = match cmd_err.status {
                    Some(_) => format!("{}: {}", cmd_err, trim_to_string(&stderr)),
                    None => cmd_err.to_string(),
                };
                Err(anyhow!(error))
            }
        }
    }

    async fn parse_detect_response(&self, response: reqwest::Response) -> anyhow::Result<String> {
        let body = response
            .json::<serde_json::Value>()
            .await?;

        match body["Content-Type"].as_str() {
            Some(mimetype) => Ok(mimetype.to_string()),
            None => Err(anyhow!("No Content-Type header found in response")),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::{Any, TypeId};
    use std::io::{Cursor, Read};

    use crate::test_utils::assert_command_successful;

    use super::*;

    #[tokio::test]
    async fn check_tika_tooling_exists() {
        let commands = [
            "which java".to_string(),
            "which tesseract".to_string(),
            format!("java -jar {} --version", TIKA_JAR),
        ];

        for cmd in commands {
            let (stdout, stderr) = assert_command_successful(cmd).await.unwrap();
            assert_ne!(stdout, "");
            assert_eq!(stderr, "");
        }
    }

    #[test]
    fn check_tika_singleton() {
        assert_eq!(tika().type_id(), TypeId::of::<Box<Tika>>());
    }

    #[tokio::test]
    async fn test_tika_text() -> anyhow::Result<()> {
        let expected_text = "\
Daily

Clean case panels, frame, and drip tray

Empty portafilter after use and rinse
with hot water before reinserting into
group

Weekly

While hot, scrub grouphead w/ brush

Backflush w/ water

Soak portafilter and basket in hot water
or cleaner

Monthly

Take off grouphead gasket and diffuser,
inspect, and clean

Backflush w/ cleaner";

        let input_path = "../resources/pdf/Espresso Machine Cleaning Guide.pdf";
        let file = tokio::fs::File::open(input_path).await?;
        let mut text = vec![];

        let output = tika().text(file, &mut text).await?;

        let text = trim_to_string(&text);
        assert!(output.status.success());
        assert_eq!(output.error, "");
        assert_eq!(text, expected_text);
        Ok(())
    }

    #[tokio::test]
    async fn test_tika_text_with_ocr() -> anyhow::Result<()> {
        let input_path = "../resources/jpg/jQuery-text.jpg";
        let file = tokio::fs::File::open(input_path).await?;
        let mut text = vec![];

        let output = tika().text(file, &mut text).await?;

        let text = trim_to_string(&text);
        assert!(output.status.success());
        assert!(output.error.contains("INFO  [main]") && output.error.contains("org.apache.tika.parser.ocr.TesseractOCRParser"));
        assert_eq!(text, "jQuery $%&U6~");
        Ok(())
    }

    #[tokio::test]
    async fn test_tika_metadata() -> anyhow::Result<()> {
        let expected_metadata = "{\"X-TIKA:Parsed-By\":[\"org.apache.tika.parser.DefaultParser\",\"org.apache.tika.parser.mbox.MboxParser\"],\"Content-Encoding\":\"windows-1252\",\"Content-Type\":\"application/mbox\"}";

        let input_path = "../resources/mbox/ubuntu-no-small.mbox";
        let file = tokio::fs::File::open(input_path).await?;

        let output = tika().metadata(file).await?;

        assert!(output.status.success());
        assert_eq!(output.error, "");
        assert_eq!(output.json, expected_metadata);
        Ok(())
    }

    #[tokio::test]
    async fn test_tika_detect() {
        let input_path = "../resources/zip/testzip.zip";
        let file = tokio::fs::File::open(input_path).await.unwrap();

        let result = tika().detect(file).await;
        assert!(result.is_ok());
        let mimetype = result.unwrap();
        assert_eq!(mimetype, "application/zip");
    }

    #[tokio::test]
    async fn test_tika_command_not_found() {
        let expected_err = "\
command failed with non-zero exit status (code 1): Exception in thread \"main\" java.net.MalformedURLException: no protocol: OOGLY BOOGLY
\tat java.base/java.net.URL.<init>(URL.java:645)
\tat java.base/java.net.URL.<init>(URL.java:541)
\tat java.base/java.net.URL.<init>(URL.java:488)
\tat org.apache.tika.cli.TikaCLI.process(TikaCLI.java:486)
\tat org.apache.tika.cli.TikaCLI.main(TikaCLI.java:256)";

        let mut input = Cursor::new(b"hello world".to_vec());
        let mut null = tokio::fs::File::open("/dev/null").await.unwrap();

        let result = tika().run(vec!["OOGLY BOOGLY"], &mut input, &mut null).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.to_string(), expected_err);
    }
}
