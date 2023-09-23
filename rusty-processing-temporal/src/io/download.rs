use bytes::Bytes;
use futures::TryStreamExt;
use tokio::sync::mpsc::Sender;

use crate::util::parse_s3_uri;
use crate::services::s3_client;

pub async fn download(source_s3_uri: String, data_sender: Sender<Bytes>) -> anyhow::Result<()> {
    let (bucket, key) = parse_s3_uri(&source_s3_uri)?;
    let object = s3_client()
        .await
        .get_object()
        .bucket(bucket)
        .key(key)
        .send();

    let mut body_stream = object.await?.body;
    while let Some(buf) = body_stream.try_next().await? {
        data_sender.send(buf).await?;
    }

    Ok(())
}
