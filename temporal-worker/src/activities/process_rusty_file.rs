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

use crate::util::{BatchEntry, ProcessOutputBatcher};

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

    /// The name of the Redis stream to send output to.
    ///
    output_stream_name: String
}

/// Output of the `process_rusty_file` activity.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessRustyFileOutput {}

/// Activity for processing a file.
///
/// This activity downloads a file from S3, processes it, and uploads the
/// result back to S3 in the form of an archive.
///
pub async fn process_rusty_file(_ctx: ActContext, input: ProcessRustyFileInput) -> anyhow::Result<ProcessRustyFileOutput> {
    info!("Processing rusty file '{:?}'", input);

    let (output_sink, outputs) = tokio::sync::mpsc::channel(100);
    let ctx = ProcessContextBuilder::new(
        input.mimetype,
        input.types,
        output_sink,
    ).build();

    let processing = tokio::spawn(processor().process(ctx, input.path));
    let output_handling = tokio::spawn(handle_outputs(
        outputs,
        input.directory,
        input.output_stream_name
    ));

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
    output_handling.await??;

    Ok(ProcessRustyFileOutput {})
}

async fn handle_outputs(
    mut outputs: Receiver<anyhow::Result<ProcessOutput>>,
    output_dir: impl AsRef<Path>,
    output_stream_name: impl AsRef<str>,
) -> anyhow::Result<()> {
    let output_dir = output_dir.as_ref();

    info!("Handling outputs");
    let mut batcher = ProcessOutputBatcher::new(output_stream_name.as_ref(), 25);
    while let Some(output) = outputs.recv().await {
        debug!("Received metadata.json: {:?}", output);

        if let Ok(output) = output.tap(log_err!("Error processing file")) {
            match output {
                ProcessOutput::Processed(_, data) => {
                    let output_path = output_dir.join(data.name);
                    copy_making_dirs(&data.path, &output_path)?;
                },

                ProcessOutput::Embedded(_, data, _) => {
                    let output_path = output_dir.join(&data.checksum).join(&data.name);
                    copy_making_dirs(&data.path, &output_path)?;

                    info!("Adding embedded file to Redis stream: {:?}", &data.path);
                    batcher.push(BatchEntry {
                        path: output_path,
                        mimetype: data.mimetype,
                        checksum: data.checksum,
                    }).await?;
                }
            }
        }
    }

    // TODO - Utilize drop trait once async capabilities are available
    batcher.flush().await?;
    Ok(())
}

fn copy_making_dirs(source_path: impl AsRef<Path>, output_path: impl AsRef<Path>) -> anyhow::Result<()> {
    let output_path = output_path.as_ref();
    fs::create_dir_all(output_path.parent().unwrap())
        .and(fs::copy(source_path.as_ref(), output_path))
        .tap(log_err!("Failed to copy file to output directory"))?;
    Ok(())
}
