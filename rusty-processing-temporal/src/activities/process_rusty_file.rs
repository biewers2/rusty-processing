use futures::{StreamExt, try_join, TryFutureExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use temporal_sdk::ActContext;
use tokio_stream::wrappers::ReceiverStream;

use rusty_processing::processing::{processor, ProcessOutput, ProcessType};

use crate::io::download::download;

#[derive(Deserialize, Debug)]
pub struct ProcessRustyFileInput {
    pub source_s3_uri: String,
    pub output_s3_uri: String,
    pub mimetype: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TmpFailureOutput {
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessRustyFileOutput {
    pub results: Vec<ProcessOutput>,
    pub failures: Vec<TmpFailureOutput>,
}

pub async fn process_rusty_file_activity(
    _ctx: ActContext,
    input: ProcessRustyFileInput,
) -> anyhow::Result<ProcessRustyFileOutput> {
    let (results, failures) = process_rusty_file(
        input.source_s3_uri,
        input.output_s3_uri,
        input.mimetype
    ).await?;

    Ok(ProcessRustyFileOutput {
        results,
        failures,
    })
}

pub async fn process_rusty_file(
    source_s3_uri: impl AsRef<str>,
    output_s3_uri: impl AsRef<str>,
    mimetype: impl Into<String>,
) -> anyhow::Result<(Vec<ProcessOutput>, Vec<TmpFailureOutput>)> {
    let mut results = vec![];
    let mut failures = vec![];

    let (dl_data_sink, source_stream) = tokio::sync::mpsc::channel(100);
    let mut source_stream = ReceiverStream::new(source_stream);

    let (process_output_sink, process_output_stream) = tokio::sync::mpsc::channel(100);
    let mut process_output_stream = ReceiverStream::new(process_output_stream);

    let dl_fut = download(source_s3_uri.as_ref().to_string(), dl_data_sink);
    let process_fut = processor().process(
        source_stream,
        process_output_sink,
        mimetype.into(),
        vec![ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf],
    );
    let recv_fut = async {
        println!("Receiving output...");
        while let Some(output) = process_output_stream.next().await {
            println!("Received output: {:?}", output);
            match output {
                Ok(output) => results.push(output),
                Err(err) => failures.push(TmpFailureOutput {
                    message: err.to_string(),
                }),
            }
        }
        anyhow::Ok(())
    };

    let dl_fut = tokio::spawn(dl_fut);
    let process_fut = tokio::spawn(process_fut);
    recv_fut.await?;
    dl_fut.await??;
    process_fut.await??;

    Ok((results, failures))
}
