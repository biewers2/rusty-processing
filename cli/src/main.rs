use std::path;
use std::path::PathBuf;
use anyhow::anyhow;

use clap::Parser;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use tap::Tap;
use tokio::sync::mpsc::{Receiver, Sender};

use processing::processing::{ProcessContextBuilder, processor, ProcessOutput, ProcessType};
use services::{ArchiveBuilder, ArchiveEntry, log_err};

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

/// The number of threads to use for handling outputs.
///
const OUTPUT_HANDLING_THREADS: usize = 1000;

#[derive(Parser, Debug)]
struct Args {
    #[arg(
        short = 'i',
        long,
        value_parser = parse_input_file
    )]
    input: path::PathBuf,

    #[arg(
        short = 'o',
        long
    )]
    output: path::PathBuf,

    #[arg(short = 'm', long)]
    mimetype: String,

    #[arg(
        short = 't',
        long,
        num_args = 0..,
        value_delimiter = ' ',
    )]
    types: Vec<ProcessType>,

    #[arg(short = 'a', long)]
    all: bool,
}

fn parse_input_file(path_str: &str) -> Result<path::PathBuf, String> {
    let path = path::PathBuf::from(path_str.to_string());
    if !path.exists() {
        return Err(format!("Path {} not found", path_str))
    }
    if !path.is_file() {
        return Err(format!("Path {} is not a file", path_str))
    }
    Ok(path)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;

    let args = Args::parse();
    let types = if args.all {
        ProcessType::all().to_vec()
    } else {
        args.types
    };

    let mut resulting_file = process(args.input, args.mimetype, types, true).await?;
    let mut output_file = tokio::fs::File::create(args.output).await?;
    tokio::io::copy(&mut resulting_file, &mut output_file).await?;

    Ok(())
}

/// Process a stream of bytes.
///
/// This function processes a stream of bytes, and returns an archive file
/// containing the metadata.json of the processing operation.
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
///     containing the metadata.json files of the processing operation.
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
/// Each metadata.json received is submitted to a thread pool to be handled on a separate thread. This allows us to
/// continuing receiving processing outputs without blocking.
///
/// Archive entries created from each metadata.json is sent to the archive entry sink.
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

/// Handle a single metadata.json of the processing operation in an asynchronous scope.
///
/// If the metadata.json should be handled recursively (i.e. `recurse = true`), then if it's embedded, the content of the embedded file
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

/// If the metadata.json is a normal metadata.json from the processing operation, then it will be used to create an archive entry.
/// If the metadata.json is an embedded file, then it will be used to create an archive entry AND also be processed.
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

/// Regardless of if the metadata.json is normal or an embedded file, both will be used to create an archive entry and no additional
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
