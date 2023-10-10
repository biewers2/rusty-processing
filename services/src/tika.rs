use std::future::Future;
use std::path::Path;

use anyhow::anyhow;
use futures::StreamExt;
use lazy_static::lazy_static;
use log::{debug, info};
use reqwest::Body;
use tokio::io::AsyncRead;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::codec::{BytesCodec, FramedRead};

use streaming::ByteStream;
use crate::config;

pub type TikaService = Box<Tika>;

lazy_static! {
    static ref TIKA: TikaService = Box::<Tika>::default();
}

pub fn tika() -> &'static TikaService {
    &TIKA
}

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
    pub async fn is_connected(&self) -> bool {
        self.http_client
            .get(self.url("/tika"))
            .send().await
            .is_ok()
    }

    pub async fn text(&self, path: impl AsRef<Path>) -> anyhow::Result<(ByteStream, impl Future<Output=anyhow::Result<()>>)> {
        info!("Using Tika to extract text");

        let input = tokio::fs::File::open(path).await?;
        let response = self.http_client
            .put(self.url("/tika"))
            .header("Accept", "text/plain")
            .header("X-Tika-Skip-Embedded", "true")
            .body(Self::body_from_input(input))
            .send().await?;
        debug!("Tika responded with {}", response.status());

        let (sink, stream) = tokio::sync::mpsc::channel(100);
        let reading = async move {
            let mut stream = response.bytes_stream();
            while let Some(bytes) = stream.next().await {
                sink.send(bytes?).await?;
            }
            anyhow::Ok(())
        };

        let stream = Box::pin(ReceiverStream::new(stream));
        Ok((stream, reading))
    }

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
    /// * `input` - The content representing the file to detect the mimetype of.
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