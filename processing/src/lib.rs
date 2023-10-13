//!
//! Library for processing files
//!
//! This library provides a framework for processing files. It also provides a default processor that can be used
//! in applications.
//!
#![warn(missing_docs)]

use std::path::PathBuf;
use anyhow::anyhow;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use mail_parser::ContentType;
use tap::Tap;
use tempfile::{NamedTempFile, TempPath};
use tokio::sync::mpsc::{Receiver, Sender};
use services::{ArchiveBuilder, ArchiveEntry, log_err};
use crate::processing::{ProcessContextBuilder, processor, ProcessOutput, ProcessType};

/// Contains the core logic and interface for processing files.
///
/// Provides the all-purpose processor that can be used to process all implemented file types.
///
pub mod processing;

pub(crate) mod text;
pub(crate) mod metadata;
pub(crate) mod pdf;
pub(crate) mod embedded;

/// The number of threads to use for handling outputs.
///
const OUTPUT_HANDLING_THREADS: usize = 1000;

lazy_static! {
    static ref RUNTIME: tokio::runtime::Runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");
}

/// Global asynchronous runtime.
///
pub fn runtime() -> &'static tokio::runtime::Runtime {
    &RUNTIME
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
pub async fn process(
    path: impl Into<PathBuf>,
    mimetype: impl Into<String>,
    types: Vec<ProcessType>,
    recurse: bool,
) -> anyhow::Result<tokio::fs::File> {
    let mimetype = mimetype.into();
    info!("Processing file with MIME type {}", mimetype);

    let (output_sink, outputs) = tokio::sync::mpsc::channel(100);
    let (archive_entry_sink, archive_entries) = tokio::sync::mpsc::channel(100);

    let ctx = ProcessContextBuilder::new(
        mimetype,
        types,
        output_sink,
    ).build();

    let processing = tokio::spawn(processor().process(ctx, path.into()));
    let output_handling = tokio::spawn(handle_outputs(
        outputs,
        archive_entry_sink,
        recurse,
    ));
    let archive = tokio::spawn(build_archive(archive_entries));

    processing.await?.map_err(|err| anyhow!(format!("{}", err)))?;
    output_handling.await??;
    info!("Finished processing file");

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
    let worker_pool = threadpool::ThreadPool::new(OUTPUT_HANDLING_THREADS);

    while let Some(output) = outputs.recv().await {
        if let Ok(output) = output.tap(log_err!("Error processing")) {
            let archive_entry_sink = archive_entry_sink.clone();
            worker_pool.execute(move || runtime().block_on(
                handle_output_asynchronously(output, recurse, archive_entry_sink)
            ));
        }
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
        Err(e) => warn!("Error processing: {:?}", e),
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
            id_chain.push(data.checksum);

            let ctx =
                ProcessContextBuilder::new(
                    data.mimetype,
                    data.types,
                    output_sink.clone(),
                )
                .id_chain(id_chain.clone())
                .build();

            if let Err(e) = processor().process(ctx, data.path.to_path_buf()).await {
                warn!("Error processing: {:?}", e);
            };

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
            id_chain.push(data.checksum);
            Ok(ArchiveEntry::new(data.name, data.path, id_chain))
        }
    }
}

/// Future for building the archive by reading from received `entries`.
///
async fn build_archive(mut entries: Receiver<ArchiveEntry>) -> anyhow::Result<std::fs::File> {
    let mut archive_builder = ArchiveBuilder::new()?;
    while let Some(archive_path) = entries.recv().await {
        debug!("Adding archive entry {:?}", archive_path);
        archive_builder.append(archive_path).await?;
    }
    archive_builder.build()
}

pub(crate) struct ProcessTypePaths {
    text: Option <TempPath>,
    metadata: Option<TempPath>,
    pdf: Option<TempPath>,
}

/// Build a tuple of paths (Text, Metadata, PDF) from a list of process types.
///
/// # Arguments
///
/// * `types` - A list of process types to build paths for.
///
/// # Returns
///
/// A tuple of paths (Text, Metadata, PDF) where each element is `Some(path)` if the corresponding
/// process type is in the list, or `None` if it is not.
///
pub(crate) fn build_paths_from_types(types: &[ProcessType]) -> std::io::Result<ProcessTypePaths> {
    let text = types.contains(&ProcessType::Text).then(temp_path).transpose()?;
    let metadata = types.contains(&ProcessType::Metadata).then(temp_path).transpose()?;
    let pdf = types.contains(&ProcessType::Pdf).then(temp_path).transpose()?;
    Ok(ProcessTypePaths { text, metadata, pdf })
}

/// Creates a temporary file and returns its path.
///
fn temp_path() -> std::io::Result<TempPath> {
    Ok(NamedTempFile::new()?.into_temp_path())
}

/// Get the MIME type from a `mail_parser::ContentType`.
///
/// # Arguments
///
/// * `content_type` - The `mail_parser::ContentType` to get the MIME type from.
///
/// # Returns
///
/// The MIME type formatted as a `String`.
///
pub fn mimetype(content_type: &ContentType) -> String {
    match (content_type.ctype(), content_type.subtype()) {
        (ctype, Some(subtype)) => format!("{}/{}", ctype, subtype),
        (ctype, None) => ctype.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use mail_parser::ContentType;
    use super::*;
    
    #[test]
    fn test_build_paths_from_types_no_types() {
        let paths = build_paths_from_types(&[]).unwrap();

        assert!(paths.text.is_none());
        assert!(paths.metadata.is_none());
        assert!(paths.pdf.is_none());
    }

    #[test]
    fn test_build_paths_from_types_all_types() {
        let types = vec![ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf];
        let paths = build_paths_from_types(&types).unwrap();

        assert!(paths.text.is_some());
        assert!(paths.metadata.is_some());
        assert!(paths.pdf.is_some());
    }

    #[test]
    fn test_mimetype_with_subtype() {
        let content_type = ContentType {
            c_type: Cow::from("text"),
            c_subtype: Some(Cow::from("plain")),
            attributes: None,
        };

        assert_eq!(mimetype(&content_type), "text/plain");
    }

    #[test]
    fn test_mimetype_without_subtype() {
        let content_type = ContentType {
            c_type: Cow::from("text"),
            c_subtype: None,
            attributes: None,
        };

        assert_eq!(mimetype(&content_type), "text");
    }
}
