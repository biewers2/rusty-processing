use std::path::Path;

use anyhow::anyhow;
use futures::StreamExt;
use lazy_static::lazy_static;
use log::{debug, info};
use reqwest::{Body, Response};
use tokio::io::{AsyncRead, AsyncWriteExt};
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::config;

/// The type of the singleton instance of the `Tika` service.
///
pub type TikaService = Box<Tika>;

lazy_static! {
    static ref TIKA: TikaService = Box::<Tika>::default();
}

/// Returns the singleton instance of the `Tika` service.
///
pub fn tika() -> &'static TikaService {
    &TIKA
}

/// The `Tika` service.
///
pub struct Tika {
    http_client: reqwest::Client,
    tika_url: String,
}

impl Default for Tika {
    fn default() -> Self {
        let host = config().get_or("TIKA_HOST", "localhost");
        let port = config().get_or("TIKA_PORT", "9998");
        let tika_url = format!("http://{}:{}", host, port);

        Self {
            http_client: reqwest::Client::new(),
            tika_url,
        }
    }
}

impl Tika {
    /// Checks if the Tika server is running.
    ///
    pub async fn is_connected(&self) -> bool {
        self.http_client
            .get(self.url("/tika"))
            .send().await
            .is_ok()
    }

    /// Extracts the text from the input file.
    ///
    /// # Arguments
    ///
    /// * `input_path` - The path to the input file.
    ///
    /// # Returns
    ///
    /// The text extracted from the input file.
    ///
    pub async fn text(&self, path: impl AsRef<Path>) -> anyhow::Result<String> {
        info!("Using Tika to extract text");

        let response = self.request_text(path).await?;
        debug!("Tika responded with {}", response.status());

        let bytes = response.bytes().await?;
        let text = String::from_utf8(bytes.to_vec())?;
        Ok(text)
    }

    /// Extracts the text from the input file and writes it to the output file.
    ///
    /// # Arguments
    ///
    /// * `input_path` - The path to the input file.
    /// * `output_path` - The path to the output text file.
    ///
    /// # Returns
    ///
    /// The text extracted from the input file.
    ///
    pub async fn text_into_file(&self, input_path: impl AsRef<Path>, output_path: impl AsRef<Path>) -> anyhow::Result<()> {
        info!("Using Tika to extract text");

        let response = self.request_text(input_path).await?;
        debug!("Tika responded with {}", response.status());

        let mut stream = response.bytes_stream();
        let mut output_file = tokio::fs::File::create(output_path.as_ref()).await?;
        while let Some(bytes) = stream.next().await {
            output_file.write_all(&bytes?).await?;
        }

        Ok(())
    }

    async fn request_text(&self, input_path: impl AsRef<Path>) -> anyhow::Result<Response> {
        let input = tokio::fs::File::open(input_path).await?;
        Ok(self.http_client
            .put(self.url("/tika"))
            .header("Accept", "text/plain")
            .header("X-Tika-Skip-Embedded", "true")
            .body(Self::body_from_input(input))
            .send().await?)
    }

    /// Extracts the metadata from the input file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the input file.
    ///
    /// # Returns
    ///
    /// The metadata extracted from the input file.
    ///
    pub async fn metadata(&self, path: impl AsRef<Path>) -> anyhow::Result<String> {
        info!("Using Tika to extract metadata");

        let input = tokio::fs::File::open(path).await?;
        let response = self.http_client
            .put(self.url("/meta"))
            .header("Accept", "application/json")
            .header("X-Tika-Skip-Embedded", "true")
            .body(Self::body_from_input(input))
            .send().await?;
        debug!("Tika responded with {}", response.status());

        Ok(response.text().await?)
    }

    /// Detects the mimetype of the input file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the input file.
    ///
    /// # Returns
    ///
    /// The mimetype of the input file.
    ///
    pub async fn detect(&self, path: impl AsRef<Path>) -> anyhow::Result<String> {
        info!("Using Tika to detect mimetype");

        let input = tokio::fs::File::open(path).await?;
        let response = self.http_client
            .put(self.url("/meta/Content-Type"))
            .header("Accept", "application/json")
            .header("X-Tika-Skip-Embedded", "true")
            .body(Self::body_from_input(input))
            .send().await?;
        debug!("Tika responded with {}", response.status());

        let mimetype = self.parse_detect_response(response).await;
        debug!("Response body result: {:?}", mimetype);

        mimetype
    }

    #[inline]
    fn url(&self, endpoint: impl AsRef<str>) -> String {
        format!("{}{}", self.tika_url, endpoint.as_ref())
    }

    #[inline]
    fn body_from_input<R>(input: R) -> Body where R: AsyncRead + Send + Sync + Unpin + 'static {
        let stream = FramedRead::new(input, BytesCodec::new());
        Body::wrap_stream(stream)
    }

    #[inline]
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

    use super::*;

    #[test]
    fn check_singleton() {
        assert_eq!(tika().type_id(), TypeId::of::<Box<Tika>>());
    }

    #[test]
    fn test_parse_detect_response() {
        // todo!()
    }
}