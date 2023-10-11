use log::{info, warn};
use serde::{Deserialize, Serialize};
use temporal_sdk::ActContext;
use tokio::sync::mpsc::Receiver;

use processing::processing::{ProcessContextBuilder, processor, ProcessOutput, ProcessType};
use streaming::async_read_to_stream;

use crate::io::S3GetObject;

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
    output_dir_s3_uri: String,

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
    s3_uri: String,

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
    // ctx: WfContext,
    _ctx: ActContext,
    input: ProcessRustyFileInput,
) -> anyhow::Result<ProcessRustyFileOutput> {
// ) -> anyhow::Result<WfExitValue<ProcessRustyFileOutput>> {
    info!("Processing rusty file: {:?}", input);

    let get_object = Box::new(S3GetObject::new(input.source_s3_uri).await?);
    let (stream, get_object) = async_read_to_stream(get_object.body)?;
    let (output_sink, outputs) = tokio::sync::mpsc::channel(100);

    info!("Building processing context");
    let ctx = ProcessContextBuilder::new(
        input.mimetype,
        PROCESS_TYPES.to_vec(),
        output_sink,
    ).build();

    info!("Starting processing threads");
    let get_object = tokio::spawn(get_object);
    let processing = tokio::spawn(processor().process(ctx, stream));
    let outputting = tokio::spawn(handle_outputs(outputs, input.output_dir_s3_uri));

    processing.await??;
    get_object.await??;
    let files = outputting.await??;

    Ok(ProcessRustyFileOutput { files })
}

async fn handle_outputs(mut outputs: Receiver<anyhow::Result<ProcessOutput>>, output_dir_s3_uri: impl AsRef<str>) -> anyhow::Result<Vec<FileInfo>> {
    let mut files = vec![];
    while let Some(output) = outputs.recv().await {
        match output {
            Ok(output) => {
                if let Some(file_info) = handle_output(output, output_dir_s3_uri.as_ref()).await? {
                    files.push(file_info);
                }
            },
            Err(e) => warn!("Error processing: {:?}", e),
        }
    }
    Ok(files)
}

async fn handle_output(output: ProcessOutput, output_dir_s3_uri: impl AsRef<str>) -> anyhow::Result<Option<FileInfo>> {
    match output {
        ProcessOutput::Processed(_, data) => {
            let s3_uri = format!("{}/{}", output_dir_s3_uri.as_ref(), data.name);
            // let file = tokio::fs::File::open(data.path).await?;

            info!("Uploading to {}", s3_uri);
            // upload(file, s3_uri).await?;

            Ok(None)
        },

        ProcessOutput::Embedded(_, data, _) => {
            let s3_uri = format!("{}/{}/{}", output_dir_s3_uri.as_ref(), data.dedupe_id, data.name);
            // let file = tokio::fs::File::open(data.path).await?;

            info!("Uploading to {}", s3_uri);
            // upload(file, &s3_uri).await?;

            Ok(Some(FileInfo {
                s3_uri,
                mimetype: data.mimetype,
                id: data.dedupe_id,
            }))
        }
    }
}
