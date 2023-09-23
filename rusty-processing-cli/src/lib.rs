use std::io::{BufReader, Read};
use bytes::Bytes;
use tokio::sync::mpsc::Sender;

pub async fn read_to_stream(source: impl Read, sink: Sender<Bytes>) -> anyhow::Result<()> {
    let mut reader = BufReader::new(source);
    let mut buf = [0; 8192];
    loop {
        let bytes_read = reader.read(&mut buf)?;
        if bytes_read == 0 {
            break;
        }
        let bytes = Bytes::copy_from_slice(&buf[..bytes_read]);
        sink.send(bytes).await?;
    }

    Ok(())
}
