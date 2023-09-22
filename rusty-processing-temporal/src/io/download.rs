use bytes::Bytes;
use futures::TryStreamExt;
use tokio::sync::mpsc::Sender;

use crate::util::parse_s3_uri::parse_s3_uri;
use crate::util::services::s3_client;

pub async fn download(source_s3_uri: String, data_sender: Sender<Bytes>) -> anyhow::Result<()>
{
    println!("Starting download");

    let (bucket, key) = parse_s3_uri(source_s3_uri.as_ref())?;
    let object = s3_client()
        .await
        .get_object()
        .bucket(bucket)
        .key(key)
        .send();

    let mut body_stream = object.await?.body;
    while let Some(buf) = body_stream.try_next().await? {
        println!("Sending {} bytes", buf.len());
        data_sender.send(buf).await?;
    }

    Ok(())
}
