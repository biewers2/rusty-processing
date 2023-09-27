use futures::executor::block_on;
use tokio::io::AsyncRead;

use crate::services::s3_client;
use crate::util::parse_s3_uri;

pub struct S3GetObject {
    pub body: Box<dyn AsyncRead + Send + Unpin>,
}

impl S3GetObject {
    pub fn new(s3_uri: String) -> anyhow::Result<Self> {
        let (bucket, key) = parse_s3_uri(&s3_uri)?;
        let object = block_on(async {
            s3_client().await
                .get_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
        })?;

        Ok(Self { body: Box::new(object.body.into_async_read()) })
    }
}
