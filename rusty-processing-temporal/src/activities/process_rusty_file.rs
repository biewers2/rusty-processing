use std::path;
use std::sync::{Arc, Mutex};
use anyhow::anyhow;

use async_recursion::async_recursion;
use futures::future::try_join_all;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use temporal_sdk::ActContext;
use tokio::fs::File;
use tokio::sync::mpsc::Sender;
use tokio::try_join;
use tokio_stream::wrappers::ReceiverStream;

use rusty_processing::common::ByteStream;
use rusty_processing::processing::{processor, ProcessOutput, ProcessOutputType, ProcessType};

use crate::io::S3GetObject;
use crate::io::upload;
use crate::services::ArchiveBuilder;
use crate::util::{path_file_name_or_random, read_to_stream};

/// Input to the `process_rusty_file` activity.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessRustyFileInput {
    /// The S3 URI of the file to process.
    ///
    source_s3_uri: String,

    /// The S3 URI of where to write the output archive to.
    ///
    output_s3_uri: String,

    /// The MIME type of the file to process.
    ///
    mimetype: String,
}

/// Struct to hold information about a failure (temporary).
///
#[derive(Debug, Serialize, Deserialize)]
pub struct TmpFailureOutput {
    message: String,
}

/// Output from the `process_rusty_file` activity.
///
#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessRustyFileOutput;

/// Activity for processing a file.
///
/// This activity downloads a file from S3, processes it, and uploads the
/// result back to S3 in the form of an archive.
///
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

/// Process a file.
///
/// This function downloads a file from S3, processes it, and uploads the
/// result back to S3 in the form of an archive.
///
/// # Arguments
///
/// * `source_s3_uri` - The S3 URI of the file to process.
/// * `output_s3_uri` - The S3 URI of where to write the output archive to.
/// * `mimetype` - The MIME type of the file to process.
///
/// # Returns
///
/// * `Ok(())` - If the file was processed successfully.
/// * `Err(_)` - If there was an error processing the file.
///
pub async fn process_rusty_file(
    source_s3_uri: impl Into<String>,
    output_s3_uri: impl Into<String>,
    mimetype: impl Into<String>,
) -> anyhow::Result<()> {
    let get_object = Box::new(S3GetObject::new(source_s3_uri.into())?);
    let (stream, get_object_fut) = read_to_stream(get_object.body).await?;

    let archive_builder = Arc::new(Mutex::new(ArchiveBuilder::new()?));
    process_recursively(stream, mimetype.into(), archive_builder.clone(), vec![]).await?;
    get_object_fut.await??;

    let archive_file = archive_builder.lock().unwrap().build()?;
    upload(archive_file, output_s3_uri.into()).await
}

/// Process content recursively by processing newly discovered embedded content.
///
/// # Arguments
///
/// * `source_stream` - The stream of bytes to process.
/// * `mimetype` - The MIME type of the content to process.
/// * `archive_builder` - The builder for the output archive.
/// * `embedded_dupe_chain` - The chain of dupe IDs for embedded content, used to structure the ZIP archive.
///
#[async_recursion]
async fn process_recursively(
    source_stream: ByteStream,
    mimetype: String,
    archive_builder: Arc<Mutex<ArchiveBuilder>>,
    embedded_dupe_chain: Vec<String>
) -> anyhow::Result<()> {
    let mut proc_futs = vec![];
    let mut failures = vec![];

    // Begin processing data
    let (process_output_sink, process_output_stream) = tokio::sync::mpsc::channel(100);
    let mut process_output_stream = Box::new(ReceiverStream::new(process_output_stream));
    proc_futs.push(tokio::spawn(
        process_stream(source_stream, process_output_sink, mimetype)
    ));

    // Begin receiving data
    while let Some(output) = process_output_stream.next().await {
        match output {
            Ok(output) => {
                let mut dupe_chain = embedded_dupe_chain.clone();
                println!("Processing output: {:?}", output);

                match output.output_type {
                    ProcessOutputType::Processed => {
                        let entry_path = build_archive_entry_path(&output.path, &dupe_chain);
                        archive_builder.lock().unwrap().add_new(entry_path)?;
                    },

                    ProcessOutputType::Embedded => {
                        dupe_chain.push(output.dupe_id);
                        let entry_path = build_archive_entry_path(&output.path, &dupe_chain);

                        let recursive_archive_builder = archive_builder.clone();
                        let file = Box::new(File::open(&output.path).await?);
                        proc_futs.push(tokio::spawn(async move {
                            let (emb_stream, emb_read_fut) = read_to_stream(file).await?;
                            let proc_fut = tokio::spawn(process_recursively(
                                emb_stream,
                                output.mimetype,
                                recursive_archive_builder,
                                dupe_chain
                            ));

                            emb_read_fut.await??;
                            proc_fut.await??;
                            Ok(())
                        }));

                        archive_builder.lock().unwrap().add_new(entry_path)?;
                    }
                };
            },
            Err(e) => failures.push(e),
        }
    }

    for fut in proc_futs {
        if let Err(e) = fut.await? {
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

fn build_archive_entry_path(local_path: impl AsRef<path::Path>, embedded_dupe_chain: &[String]) -> path::PathBuf {
    let mut path = path::PathBuf::new();
    for dupe_id in embedded_dupe_chain {
        path.push(dupe_id);
    }

    let name = path_file_name_or_random(local_path);
    path.push(name);

    path
}