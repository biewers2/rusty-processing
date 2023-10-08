use serde::{Deserialize, Serialize};
use temporal_sdk::ActContext;

use streaming::async_read_to_stream;
use processing::process_rusty_stream;
use processing::processing::ProcessType;

use crate::io::{S3GetObject, upload};

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

    /// Whether to process discovered embedded files.
    ///
    recurse: bool,
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
    let get_object = Box::new(S3GetObject::new(input.source_s3_uri)?);
    let (stream, get_object_fut) = async_read_to_stream(get_object.body)?;
    let get_object_fut = tokio::spawn(get_object_fut);

    let archive_file = process_rusty_stream(
        stream,
        input.mimetype,
        PROCESS_TYPES.to_vec(),
        input.recurse,
    ).await?;
    get_object_fut.await??;

    upload(archive_file, input.output_s3_uri).await?;

    Ok(ProcessRustyFileOutput {})
}
