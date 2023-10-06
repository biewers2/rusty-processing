use serde::{Deserialize, Serialize};
use temporal_sdk::ActContext;
use tokio::sync::mpsc::{Receiver, Sender};

use processing::io::{ByteStream, runtime};
use processing::processing::{ProcessContextBuilder, processor, ProcessOutput, ProcessType};
use processing::io::async_read_to_stream;

use crate::io::{S3GetObject, upload};
use crate::services::{ArchiveBuilder, ArchiveEntry};

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

    recurse: bool,
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
        input.mimetype,
        input.recurse,
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
    recurse: bool,
) -> anyhow::Result<()> {
    let get_object = Box::new(S3GetObject::new(source_s3_uri.into())?);
    let (stream, get_object_fut) = async_read_to_stream(get_object.body)?;
    let get_object_fut = tokio::spawn(get_object_fut);

    let archive_file = process_rusty_stream(
        stream,
        mimetype,
        recurse,
    ).await?;
    get_object_fut.await??;

    upload(archive_file, output_s3_uri.into()).await?;

    Ok(())
}

/// Process a stream of bytes.
///
/// This function processes a stream of bytes, and returns an archive file
/// containing the output of the processing operation.
///
/// # Arguments
///
/// * `stream` - The stream of bytes to process.
/// * `mimetype` - The MIME type the stream of bytes represents.
/// * `process_recursively` - Whether to process embedded files recursively.
///
/// # Returns
///
/// * `Ok(File)` - If the stream of bytes was processed successfully, where `File` is the file of the created archive
///     containing the output files of the processing operation.
/// * `Err(_)` - If there was an error processing the stream of bytes.
///
pub async fn process_rusty_stream(
    stream: ByteStream,
    mimetype: impl Into<String>,
    recurse: bool,
) -> anyhow::Result<tokio::fs::File> {
    let (output_sink, outputs) = tokio::sync::mpsc::channel(100);
    let (archive_entry_sink, archive_entries) = tokio::sync::mpsc::channel(100);

    let ctx = ProcessContextBuilder::new(
        mimetype.into(),
        PROCESS_TYPES.to_vec(),
        output_sink,
    ).build();

    let processing = tokio::spawn(processor().process(ctx, stream));
    let output_handling = tokio::spawn(handle_outputs(
        outputs,
        archive_entry_sink,
        recurse,
    ));
    let archive = tokio::spawn(build_archive(archive_entries));

    processing.await??;
    output_handling.await??;

    let file = archive.await??;
    Ok(tokio::fs::File::from(file))
}

/// Handle the outputs of the processing operation asynchronously.
///
/// Each output received is submitted to a thread pool to be handled on a separate thread. This allows us to
/// continuing receiving processing outputs without blocking.
///
/// Archive entries created from each output is sent to the archive entry sink.
///
async fn handle_outputs(
    mut outputs: Receiver<anyhow::Result<ProcessOutput>>,
    archive_entry_sink: Sender<ArchiveEntry>,
    recurse: bool,
) -> anyhow::Result<()> {
    let worker_pool = threadpool::ThreadPool::new(100);

    while let Some(output) = outputs.recv().await {
        match output {
            Ok(output) => {
                let archive_entry_sink = archive_entry_sink.clone();
                worker_pool.execute(move || runtime().block_on(
                    handle_output_asynchronously(output, recurse, archive_entry_sink)
                ));
            },
            Err(e) => { eprintln!("Error processing: {:?}", e); },
        };
    }

    worker_pool.join();
    Ok(())
}

/// Handle a single output of the processing operation in an asynchronous scope.
///
/// If the output should be handled recursively (i.e. `recurse = true`), then if it's embedded, the content of the embedded file
/// will also be processed. Otherwise, it will be added as an archive entry and no more processing will occur.
///
async fn handle_output_asynchronously(output: ProcessOutput, recurse: bool, archive_entry_sink: Sender<ArchiveEntry>) {
    let archive_entry = if recurse {
        handle_process_output_recursively(output).await
    } else {
        handle_process_output(output).await
    };

    match archive_entry {
        Ok(archive_entry) => archive_entry_sink.send(archive_entry).await.unwrap(),
        Err(e) => eprintln!("Error processing: {:?}", e),
    }
}

/// If the output is a normal output from the processing operation, then it will be used to create an archive entry.
/// If the output is an embedded file, then it will be used to create an archive entry AND also be processed.
///
async fn handle_process_output_recursively(output: ProcessOutput) -> anyhow::Result<ArchiveEntry> {
    match output {
        ProcessOutput::Processed(state, data) => {
            Ok(ArchiveEntry::new(data.name, data.path, state.id_chain))
        },

        ProcessOutput::Embedded(state, data, output_sink) => {
            let mut id_chain = state.id_chain;
            id_chain.push(data.dedupe_id);

            let ctx_builder = ProcessContextBuilder::new(
                data.mimetype,
                PROCESS_TYPES.to_vec(),
                output_sink.clone(),
            );
            let ctx = ctx_builder.id_chain(id_chain.clone()).build();

            let file = Box::new(tokio::fs::File::open(&data.path).await?);
            let (emb_stream, emb_read_fut) = async_read_to_stream(file) ?;
            let emb_read_fut = tokio::spawn(emb_read_fut);

            processor().process(ctx, emb_stream).await?;
            emb_read_fut.await??;

            Ok(ArchiveEntry::new(data.name, data.path, id_chain))
        }
    }
}

/// Regardless of if the output is normal or an embedded file, both will be used to create an archive entry and no additional
/// processing will occur.
///
async fn handle_process_output(output: ProcessOutput) -> anyhow::Result<ArchiveEntry> {
    match output {
        ProcessOutput::Processed(state, data) => {
            Ok(ArchiveEntry::new(data.name, data.path, state.id_chain))
        },

        ProcessOutput::Embedded(state, data, _) => {
            let mut id_chain = state.id_chain;
            id_chain.push(data.dedupe_id);
            Ok(ArchiveEntry::new(data.name, data.path, id_chain))
        }
    }
}

/// Future for building the archive by reading from received `entries`.
///
async fn build_archive(mut entries: Receiver<ArchiveEntry>) -> anyhow::Result<std::fs::File> {
    let mut archive_builder = ArchiveBuilder::new()?;
    while let Some(archive_path) = entries.recv().await {
        println!("Adding to archive: {:?}", archive_path);
        archive_builder.append(archive_path).await?;
    }
    archive_builder.build()
}