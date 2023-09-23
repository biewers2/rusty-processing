use std::fs::File;
use std::io::{BufRead, BufReader, Seek};
use std::os::unix::fs::MetadataExt;
use std::path;
use std::sync::{Arc, Mutex};

use async_recursion::async_recursion;
use bytes::Bytes;
use futures::StreamExt;
use futures::future::try_join_all;
use serde::{Deserialize, Serialize};
use temporal_sdk::ActContext;
use tokio::sync::mpsc::Sender;
use tokio_stream::wrappers::ReceiverStream;

use rusty_processing::common::ByteStream;
use rusty_processing::processing::{processor, ProcessOutput, ProcessOutputType, ProcessType};

use crate::io::download::download;
use crate::io::upload::upload;
use crate::services::ZipBuilder;
use crate::util::{path_file_name_or_random, read_to_stream};

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
pub struct ProcessRustyFileOutput;

pub async fn process_rusty_file_activity(
    _ctx: ActContext,
    input: ProcessRustyFileInput,
) -> anyhow::Result<ProcessRustyFileOutput> {
    process_rusty_file(
        input.source_s3_uri,
        input.output_s3_uri,
        input.mimetype
    ).await?;

    Ok(ProcessRustyFileOutput {})
}

pub async fn process_rusty_file(
    source_s3_uri: String,
    output_s3_uri: String,
    mimetype: String,
) -> anyhow::Result<()> {
    let (dl_data_sink, source_stream) = tokio::sync::mpsc::channel(100);
    let source_stream = ReceiverStream::new(source_stream);

    println!("Starting download");
    let dl_fut = tokio::spawn(
        download(source_s3_uri, dl_data_sink)
    );

    println!("Starting processing/zipping");
    let zip_builder = Arc::new(Mutex::new(ZipBuilder::new()?));
    process_recursively(source_stream, mimetype, zip_builder.clone(), vec![]).await?;
    dl_fut.await??;

    let zip_file = zip_builder.lock().unwrap().build()?;
    println!("Zip file size: {}", zip_file.metadata()?.size());

    upload(zip_file, output_s3_uri).await
}

#[async_recursion]
async fn process_recursively(
    source_stream: ReceiverStream<Bytes>,
    mimetype: String,
    zip_builder: Arc<Mutex<ZipBuilder>>,
    embedded_dupe_chain: Vec<String>
) -> anyhow::Result<()> {
    let mut proc_futs = vec![];
    let mut failures = vec![];

    // Begin processing data
    let (process_output_sink, process_output_stream) = tokio::sync::mpsc::channel(100);
    let mut process_output_stream = ReceiverStream::new(process_output_stream);
    proc_futs.push(tokio::spawn(
        process_stream(Box::new(source_stream), process_output_sink, mimetype)
    ));

    // Begin receiving data
    while let Some(output) = process_output_stream.next().await {
        match output {
            Ok(output) => {
                let mut dupe_chain = embedded_dupe_chain.clone();

                match output.output_type {
                    ProcessOutputType::Processed => {
                        let entry_path = zip_entry_path(&output.path, &dupe_chain);
                        zip_builder.lock().unwrap().add_new(entry_path)?;
                    },

                    ProcessOutputType::Embedded => {
                        dupe_chain.push(output.dupe_id);
                        let entry_path = zip_entry_path(&output.path, &dupe_chain);

                        let (emb_sink, emb_stream) = tokio::sync::mpsc::channel(100);
                        let emb_stream = ReceiverStream::new(emb_stream);

                        proc_futs.push(tokio::spawn(
                            process_recursively(emb_stream, output.mimetype, zip_builder.clone(), dupe_chain)
                        ));

                        let read_fut = tokio::spawn(async {
                            let file = File::open(output.path)?;
                            read_to_stream(file, emb_sink).await?;
                            anyhow::Ok(())
                        });

                        zip_builder.lock().unwrap().add_new(entry_path)?;
                        read_fut.await??;
                    }
                };
            },
            Err(e) => failures.push(e),
        }
    }

    for result in try_join_all(proc_futs).await? {
        if let Err(e) = result {
            failures.push(e);
        }
    }

    for failure in failures {
        eprintln!("Failure: {}", failure);
    }

    Ok(())
}

async fn process_stream(
    stream: ByteStream,
    output_sink: Sender<anyhow::Result<ProcessOutput>>,
    mimetype: String,
) -> anyhow::Result<()> {
    processor().process(
        stream,
        output_sink,
        mimetype,
        vec![ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf],
    ).await
}

fn zip_entry_path(local_path: impl AsRef<path::Path>, embedded_dupe_chain: &[String]) -> path::PathBuf {
    let mut path = path::PathBuf::new();
    for dupe_id in embedded_dupe_chain {
        path.push(dupe_id);
    }

    let name = path_file_name_or_random(local_path);
    path.push(name);

    path
}