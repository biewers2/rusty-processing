use std::path;
use std::path::{Path, PathBuf};
use anyhow::anyhow;

use clap::Parser;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use tap::Tap;
use tempfile::{NamedTempFile, tempfile, TempPath};
use tokio::sync::mpsc::{Receiver, Sender};

use processing::processing::{ProcessContextBuilder, processor, ProcessOutput, ProcessType};
use services::{ArchiveBuilder, log_err};

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

    process(args.input, args.output, args.mimetype, types, true).await?;

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
    input_path: PathBuf,
    output_path: PathBuf,
    mimetype: String,
    types: Vec<ProcessType>,
    recurse: bool,
) -> anyhow::Result<()> {
    info!("Processing file with MIME type {}", &mimetype);

    let (output_sink, outputs) = tokio::sync::mpsc::channel(100);
    let (archive_entry_sink, archive_entries) = tokio::sync::mpsc::channel(100);

    let ctx = ProcessContextBuilder::new(
        mimetype,
        types,
        output_sink,
    ).build();

    let processing = tokio::spawn(processor().process(ctx, input_path.into()));
    let output_handling = tokio::spawn(handle_outputs(
        outputs,
        archive_entry_sink,
        recurse,
    ));
    let archive = tokio::spawn(build_archive(archive_entries, output_path));

    processing.await?.map_err(|err| anyhow!(format!("{}", err)))?;
    output_handling.await??;
    info!("Finished processing file");

    archive.await??;
    Ok(())
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
    archive_entry_sink: Sender<(TempPath, PathBuf)>,
    recurse: bool,
) -> anyhow::Result<()> {
    let worker_pool = threadpool::ThreadPool::new(OUTPUT_HANDLING_THREADS);

    while let Some(output) = outputs.recv().await {
        if let Ok(output) = output.tap(log_err!("Error processing")) {
            let archive_entry_sink = archive_entry_sink.clone();
            worker_pool.execute(move || runtime().block_on(
                handle_process_output(output, archive_entry_sink, recurse)
            ));
        }
    }

    worker_pool.join();
    Ok(())
}

/// Regardless of if the metadata.json is normal or an embedded file, both will be used to create an archive entry and no additional
/// processing will occur.
///
async fn handle_process_output(
    output: ProcessOutput,
    archive_entry_sink: Sender<(TempPath, PathBuf)>,
    recurse: bool
) {
    let archive_entry: anyhow::Result<(TempPath, PathBuf)> = match output {
        ProcessOutput::Processed(state, data) => {
            let archive_path = build_archive_path(state.id_chain, data.name).await;
            Ok((data.path, archive_path))
        },

        ProcessOutput::Embedded(state, data, output_sink) => {
            let mut id_chain = state.id_chain;
            id_chain.push(data.checksum);

            if recurse {
                let ctx = ProcessContextBuilder::new(data.mimetype, data.types, output_sink.clone())
                    .id_chain(id_chain.clone())
                    .build();
                if let Err(e) = processor().process(ctx, data.path.to_path_buf()).await {
                    warn!("Error processing: {:?}", e);
                };
            }

            let archive_path = build_archive_path(id_chain, data.name).await;
            Ok((data.path, archive_path))
        }
    };

    match archive_entry {
        Ok(archive_entry) => archive_entry_sink.send(archive_entry).await.unwrap(),
        Err(e) => warn!("Error processing: {:?}", e),
    }
}

/// Future for building the archive by reading from received `entries`.
///
async fn build_archive(mut entries: Receiver<(TempPath, PathBuf)>, output_path: PathBuf) -> anyhow::Result<()> {
    let file = std::fs::File::create(output_path)?;
    let mut archive_builder = ArchiveBuilder::new(file)?;
    while let Some((path, zip_path)) = entries.recv().await {
        debug!("Adding archive entry {:?}", zip_path);
        archive_builder.push(path, zip_path)?;
    }
    let _ = archive_builder.build()?;
    Ok(())
}

async fn build_archive_path(id_chain: impl AsRef<[String]>, name: impl AsRef<str>) -> PathBuf {
    let mut path = PathBuf::new();
    for id in id_chain.as_ref() {
        path.push(id);
    }
    path.push(name.as_ref());
    path
}
