use std::future::Future;

use anyhow::anyhow;
use futures::StreamExt;
use lazy_static::lazy_static;
use reqwest::Body;
use tokio::io::AsyncRead;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::codec::{BytesCodec, FramedRead};

use streaming::ByteStream;

pub type TikaService = Box<Tika>;

lazy_static! {
    static ref TIKA: TikaService = Box::<Tika>::default();
}

pub fn tika() -> &'static TikaService {
    &TIKA
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
    pub async fn text<R>(&self, input: R) -> anyhow::Result<(ByteStream, impl Future<Output=anyhow::Result<()>>)>
        where R: AsyncRead + Send + Sync + Unpin + 'static
    {
        let response = self.http_client
            .put("http://localhost:9998/tika")
            .header("Accept", "text/plain")
            .header("X-Tika-Skip-Embedded", "true")
            .body(Self::body_from_input(input))
            .send().await?;

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

    pub async fn metadata<R>(&self, input: R) -> anyhow::Result<String>
        where R: AsyncRead + Send + Sync + Unpin + 'static
    {
        let response = self.http_client
            .put("http://localhost:9998/meta")
            .header("Accept", "application/json")
            .header("X-Tika-Skip-Embedded", "true")
            .body(Self::body_from_input(input))
            .send().await?;
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
    pub async fn detect<R>(&self, input: R) -> anyhow::Result<String>
        where R: AsyncRead + Send + Sync + Unpin + 'static
    {
        let response = self.http_client
            .put("http://localhost:9998/meta/Content-Type")
            .header("Accept", "application/json")
            .header("X-Tika-Skip-Embedded", "true")
            .body(Self::body_from_input(input))
            .send().await?;
        let mimetype = self.parse_detect_response(response).await?;
        Ok(mimetype)
    }

    fn body_from_input<R>(input: R) -> Body where R: AsyncRead + Send + Sync + Unpin + 'static {
        let stream = FramedRead::new(input, BytesCodec::new());
        Body::wrap_stream(stream)
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

// TODO | these should be moved to the `tests` directory, as they're integration tests.
// TODO | `parse_detect_response` should still be tested here
#[cfg(test)]
mod tests {
    use std::any::{Any, TypeId};

    use streaming::stream_to_string;

    use super::*;

    #[tokio::test]
    async fn test_tika_server_connection() {
        let client = reqwest::Client::new();
        let response = client.get("http://localhost:9998/tika").send().await;

        assert!(response.is_ok());
        let body = response.unwrap().text().await.unwrap();
        assert_eq!(body, "This is Tika Server (Apache Tika 2.9.0). Please PUT\n");
    }

    #[test]
    fn check_tika_singleton() {
        assert_eq!(tika().type_id(), TypeId::of::<Box<Tika>>());
    }

    #[tokio::test]
    async fn test_tika_text() -> anyhow::Result<()> {
        let expected_text = "
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

Backflush w/ cleaner


";

        let input_path = "../resources/pdf/Espresso Machine Cleaning Guide.pdf";
        let file = tokio::fs::File::open(input_path).await?;

        let (stream, streaming) = tika().text(file).await?;
        let streaming = tokio::spawn(streaming);

        let text = stream_to_string(stream).await;
        streaming.await??;

        assert_eq!(text, expected_text);
        Ok(())
    }

    #[tokio::test]
    async fn test_tika_text_with_ocr() -> anyhow::Result<()> {
        let input_path = "../resources/jpg/jQuery-text.jpg";
        let file = tokio::fs::File::open(input_path).await?;

        let (stream, streaming) = tika().text(file).await?;
        let streaming = tokio::spawn(streaming);

        let text = stream_to_string(stream).await;
        streaming.await??;

        assert_eq!(text, "jQuery $%&U6~\n\n\n");
        Ok(())
    }

    #[tokio::test]
    async fn test_tika_metadata() -> anyhow::Result<()> {
        let expected_metadata = "\
{\
\"X-TIKA:Parsed-By\":[\"org.apache.tika.parser.DefaultParser\",\"org.apache.tika.parser.mbox.MboxParser\"],\
\"X-TIKA:Parsed-By-Full-Set\":[\"org.apache.tika.parser.DefaultParser\",\"org.apache.tika.parser.mbox.MboxParser\"],\
\"Content-Encoding\":\"windows-1252\",\
\"language\":\"\",\
\"Content-Type\":\"application/mbox\"\
}";

        let input_path = "../resources/mbox/ubuntu-no-small.mbox";
        let file = tokio::fs::File::open(input_path).await?;

        let metadata = tika().metadata(file).await?;

        assert_eq!(metadata, expected_metadata);
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
}
