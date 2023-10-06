//!
//! Library for processing files
//!
//! This library provides a framework for processing files. It also provides a default processor that can be used
//! in applications.
//!
#![warn(missing_docs)]

use tokio::sync::mpsc::{Receiver, Sender};
use services::{ArchiveBuilder, ArchiveEntry};
use crate::io::{async_read_to_stream, ByteStream, runtime};
use crate::processing::{ProcessContextBuilder, processor, ProcessOutput, ProcessType};

/// Contains the core logic and interface for processing files.
///
/// Provides the all-purpose processor that can be used to process all implemented file types.
///
pub mod processing;

/// Contains I/O related functionality.
///
pub mod io;

pub(crate) mod application {
    #[cfg(feature = "mail")]
    pub mod mbox;

    #[cfg(feature = "archive")]
    pub mod zip;
}

#[cfg(feature = "mail")]
pub(crate) mod message {
    pub mod rfc822;
}

pub(crate) mod workspace;

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
    types: Vec<ProcessType>,
    recurse: bool,
) -> anyhow::Result<tokio::fs::File> {
    let (output_sink, outputs) = tokio::sync::mpsc::channel(100);
    let (archive_entry_sink, archive_entries) = tokio::sync::mpsc::channel(100);

    let ctx = ProcessContextBuilder::new(
        mimetype.into(),
        types,
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
                data.types,
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
        archive_builder.append(archive_path).await?;
    }
    archive_builder.build()
}

#[cfg(test)]
pub mod test_utils {
    use std::io::Read;
    use std::path;
    use bytes::Bytes;
    use rand::Rng;
    use tokio::io::AsyncReadExt;
    use crate::io::ByteStream;

    pub fn read_contents(path: &str) -> anyhow::Result<Vec<u8>> {
        let mut content = vec![];
        std::fs::File::open(path::PathBuf::from(path))?.read_to_end(&mut content)?;
        Ok(content)
    }

    pub fn byte_stream_from_string(value: impl Into<String>) -> ByteStream {
        let bytes = Bytes::from(value.into());
        Box::pin(async_stream::stream! { yield bytes })
    }

    pub async fn byte_stream_from_fs(path: path::PathBuf) -> anyhow::Result<ByteStream> {
        let file = tokio::fs::File::open(path).await.unwrap();
        let mut reader = tokio::io::BufReader::new(file);

        let mut buf = vec![];
        reader.read_to_end(&mut buf).await?;
        let bytes = Bytes::from(buf);
        let stream = Box::pin(async_stream::stream! { yield bytes });

        Ok(stream)
    }

    pub fn random_bytes(len: usize) -> Box<Vec<u8>> {
        let mut rng = rand::thread_rng();
        Box::new((0..len).map(|_| rng.gen()).collect::<Vec<u8>>())
    }

    pub fn random_byte_stream(len: usize) -> ByteStream {
        let bytes = Bytes::from(*random_bytes(len));
        Box::pin(async_stream::stream! { yield bytes })
    }
}