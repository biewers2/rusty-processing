use std::path;

use serde::{Deserialize, Serialize};
use temporal_sdk::ActContext;
use tokio::fs::File;

use rusty_processing::common::ByteStream;
use rusty_processing::processing::{ProcessContextBuilder, processor, ProcessOutput, ProcessType};

use crate::io::{S3GetObject, upload};
use crate::services::ArchiveBuilder;
use crate::util::{path_file_name_or_random, read_to_stream};

static PROCESS_TYPES: [ProcessType; 3] = [ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf];

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

/// Process a file in S3.
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
    let (stream, get_object_fut) = read_to_stream(get_object.body)?;
    let get_object_fut = tokio::spawn(get_object_fut);

    let archive_file = process_rusty_stream(
        stream,
        mimetype
    ).await?;
    get_object_fut.await??;

    upload(archive_file, output_s3_uri.into()).await?;

    Ok(())
}

async fn process_rusty_stream(
    stream: ByteStream,
    mimetype: impl Into<String>,
) -> anyhow::Result<File> {
    let (output_sink, mut output_rx) = tokio::sync::mpsc::channel(100);
    let (archive_path_sink, mut archive_paths) = tokio::sync::mpsc::channel::<path::PathBuf>(100);

    let ctx = ProcessContextBuilder::new(
        mimetype.into(),
        PROCESS_TYPES.to_vec(),
        output_sink,
    ).build();

    let process_fut = tokio::spawn(processor().process(ctx, stream));

    let handle_output_fut = tokio::spawn(async move {
        let pool = threadpool::ThreadPool::new(100);

        while let Some(output) = output_rx.recv().await {
            match output {
                Ok(output) => {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .build()?;

                    let archive_path_sink = archive_path_sink.clone();

                    pool.execute(move || rt.block_on(async {
                        match handle_process_output(output).await {
                            Ok(path) => {
                                println!("Adding to archive: {:?}", path);
                                archive_path_sink.send(path).await.unwrap();
                            },
                            Err(e) => eprintln!("Error processing: {:?}", e),
                        }
                    }));
                },
                Err(e) => { eprintln!("Error processing: {:?}", e); },
            };
        }

        pool.join();
        anyhow::Ok(())
    });

    let archive_fut = tokio::spawn(async move {
        let mut archive_builder = ArchiveBuilder::new()?;
        while let Some(archive_path) = archive_paths.recv().await {
            archive_builder.add_new(&archive_path)?;
        }
        archive_builder.build()
    });

    process_fut.await??;
    handle_output_fut.await??;

    let file = archive_fut.await??;
    Ok(File::from(file))
}

async fn handle_process_output(output: ProcessOutput) -> anyhow::Result<path::PathBuf> {
    match output {
        ProcessOutput::Processed(state, data) => {
            Ok(build_archive_entry_path(data.path, &state.id_chain))
        }

        ProcessOutput::Embedded(state, data, output_sink) => {
            let mut id_chain = state.id_chain;
            id_chain.push(data.dupe_id);
            let entry_path = build_archive_entry_path(&data.path, &id_chain);

            let ctx = ProcessContextBuilder::new(
                data.mimetype,
                PROCESS_TYPES.to_vec(),
                output_sink
            ).build();

            let file = Box::new(File::open(&data.path).await?);
            let (emb_stream, emb_read_fut) = read_to_stream(file)?;
            let emb_read_fut = tokio::spawn(emb_read_fut);

            println!("Processing embedded: {:?}", entry_path);
            processor().process(ctx, emb_stream).await?;
            emb_read_fut.await??;

            Ok(entry_path)
        }
    }
}

fn build_archive_entry_path(local_path: impl AsRef<path::Path>, embedded_dupe_chain: &[String]) -> path::PathBuf {
    let mut path = path::PathBuf::new();
    for dupe_id in embedded_dupe_chain {
        path.push(dupe_id);
    }
    path.push(path_file_name_or_random(local_path));
    path
}