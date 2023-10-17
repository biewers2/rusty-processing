use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Error};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use tap::Tap;
use temporal_sdk::{ActContext, NonRetryableActivityError};
use tokio::sync::mpsc::Receiver;

use processing::processing::{ProcessContextBuilder, ProcessingError, processor, ProcessOutput, ProcessType};
use services::log_err;

/// Input to the `process_rusty_file` activity.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessRustyFileInput {
    /// The local path to the file to process.
    ///
    path: PathBuf,

    /// The local path to the directory where output files should be written to.
    ///
    directory: PathBuf,

    /// The MIME type of the file to process.
    ///
    mimetype: String,

    /// The types of output to generate.
    ///
    types: Vec<ProcessType>,
}

/// Output from the `process_rusty_file` activity.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessRustyFileOutput {
    /// Files that were processed.
    ///
    /// These files are created during processing and should not be treated as embedded.
    ///
    processed_files: Vec<FileInfo>,

    /// Files that were embedded in the original file.
    ///
    embedded_files: Vec<FileInfo>,
}

/// Information about a file.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    /// The path of the file on the local machine.
    ///
    path: PathBuf,

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
    info!("Processing rusty file '{:?}'", input);

    let (output_sink, outputs) = tokio::sync::mpsc::channel(100);
    let ctx = ProcessContextBuilder::new(
        input.mimetype,
        input.types,
        output_sink,
    ).build();

    let processing = tokio::spawn(processor().process(ctx, input.path));
    let files = tokio::spawn(handle_outputs(outputs, input.directory));

    processing.await?
        .tap(log_err!("Failed to process file"))
        .map_err(|err| {
            match err {
                ProcessingError::UnsupportedMimeType(_) => {
                    error!("Retryable error: {}", err);
                    Error::from(NonRetryableActivityError(anyhow!(format!("{}", err))))
                },
                ProcessingError::Unexpected(err) => {
                    error!("Unexpected error: {:?}", err);
                    err
                }
            }
        })?;

    let (processed, embedded) = files.await??;
    Ok(ProcessRustyFileOutput {
        processed_files: processed,
        embedded_files: embedded,
    })
}

async fn handle_outputs(
    mut outputs: Receiver<anyhow::Result<ProcessOutput>>,
    output_dir: impl AsRef<Path>
) -> anyhow::Result<(Vec<FileInfo>, Vec<FileInfo>)> {
    info!("Handling outputs");
    let output_dir = output_dir.as_ref();

    let mut processed = vec![];
    let mut embedded = vec![];
    while let Some(output) = outputs.recv().await {
        debug!("Received metadata.json: {:?}", output);

        if let Ok(output) = output.tap(log_err!("Error processing file")) {
            match output {
                ProcessOutput::Processed(_, data) => {
                    let output_path = output_dir.join(data.name);
                    fs::create_dir_all(&output_path.parent().unwrap())
                        .and(fs::copy(&data.path, &output_path))
                        .tap(log_err!("Failed to copy file to output directory"))?;

                    processed.push(FileInfo {
                        path: output_path,
                        mimetype: data.mimetype,
                        id: data.checksum,
                    })
                },

                ProcessOutput::Embedded(_, data, _) => {
                    let output_path = output_dir.join(&data.checksum).join(&data.name);
                    fs::create_dir_all(&output_path.parent().unwrap())
                        .and(fs::copy(&data.path, &output_path))
                        .tap(log_err!("Failed to copy file to output directory"))?;

                    embedded.push(FileInfo {
                        path: output_path,
                        mimetype: data.mimetype,
                        id: data.checksum,
                    })
                }
            }
        }
    }

    Ok((processed, embedded))
}
