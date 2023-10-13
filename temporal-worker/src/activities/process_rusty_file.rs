use std::path::{Path, PathBuf};

use anyhow::{anyhow, Error};
use lazy_static::lazy_static;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use tap::Tap;
use tempfile::TempPath;
use temporal_sdk::{ActContext, NonRetryableActivityError};
use tokio::sync::mpsc::Receiver;

use processing::processing::{ProcessContextBuilder, ProcessingError, processor, ProcessOutput, ProcessType};
use processing::runtime;
use services::log_err;

use crate::activities::download;
use crate::activities::upload::upload;

static PROCESS_TYPES: [ProcessType; 3] = [ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf];

lazy_static! {
    static ref THREAD_POOL: threadpool::ThreadPool = threadpool::ThreadPool::new(100);
}

/// Input to the `process_rusty_file` activity.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessRustyFileInput {
    /// The S3 URI of the file to process.
    ///
    source_s3_uri: PathBuf,

    /// The S3 URI of where to write the output archive to.
    ///
    output_dir_s3_uri: PathBuf,

    /// The MIME type of the file to process.
    ///
    mimetype: String,
}

/// Output from the `process_rusty_file` activity.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessRustyFileOutput {
    files: Vec<FileInfo>,
}

/// Information about a file.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    /// The S3 URI of the file.
    ///
    s3_uri: PathBuf,

    /// The MIME type of the file.
    ///
    mimetype: String,

    /// Unique identification for the file.
    ///
    id: String,
}

/// Activity for processing a file.
///
/// This activity downloads a file from S3, processes it, and uploads the
/// result back to S3 in the form of an archive.
///
pub async fn process_rusty_file(
    _ctx: ActContext,
    input: ProcessRustyFileInput,
) -> anyhow::Result<ProcessRustyFileOutput> {
    info!("Processing rusty file: {:?}", input);

    let (output_sink, outputs) = tokio::sync::mpsc::channel(100);
    let ctx = ProcessContextBuilder::new(
        input.mimetype,
        PROCESS_TYPES.to_vec(),
        output_sink,
    ).build();

    let s3_uri = input.source_s3_uri;
    let path = download(&s3_uri).await
        .tap(log_err!("Failed to download from S3 URI {}", s3_uri.to_string_lossy()))?;

    let processing = tokio::spawn(processor().process(ctx, path.to_path_buf()));
    let files = tokio::spawn(handle_outputs(outputs, input.output_dir_s3_uri));

    processing.await?
        .tap(log_err!("Failed to process file {}", s3_uri.to_string_lossy()))
        .map_err(|err| {
            match err {
                ProcessingError::UnsupportedMimeType(_) => {
                    Error::from(NonRetryableActivityError(anyhow!(format!("{}", err))))
                },
                ProcessingError::Unexpected(err) => err
            }
        })?;

    let files = files.await?;
    Ok(ProcessRustyFileOutput { files })
}

async fn handle_outputs(
    mut outputs: Receiver<anyhow::Result<ProcessOutput>>,
    output_dir_s3_uri: impl AsRef<Path>
) -> Vec<FileInfo> {
    info!("Handling outputs");

    let mut files = vec![];
    while let Some(output) = outputs.recv().await {
        debug!("Received output: {:?}", output);

        if let Ok(output) = output.tap(log_err!("Error processing file")) {
            if let Some(file_info) = handle_output(output, &output_dir_s3_uri).await {
                files.push(file_info);
            }
        }
    }

    files
}

async fn handle_output(
    output: ProcessOutput,
    output_dir_s3_uri: impl AsRef<Path>
) -> Option<FileInfo> {
    match output {
        ProcessOutput::Processed(_, data) => {
            let s3_uri = output_dir_s3_uri.as_ref().join(data.name);
            submit_upload(data.path, s3_uri);
            None
        },

        ProcessOutput::Embedded(_, data, _) => {
            let s3_uri = output_dir_s3_uri.as_ref().join(&data.checksum).join(&data.name);
            submit_upload(data.path, &s3_uri);
            Some(FileInfo {
                s3_uri,
                mimetype: data.mimetype,
                id: data.checksum,
            })
        }
    }
}

fn submit_upload(path: TempPath, s3_uri: impl Into<PathBuf>) {
    let s3_uri = s3_uri.into();
    THREAD_POOL.execute(move ||
        runtime().block_on(upload(path, s3_uri))
            .tap(log_err!("Failed to upload file"))
            .unwrap()
    );
}
