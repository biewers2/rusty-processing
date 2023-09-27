use std::path;

use clap::Parser;
use tokio::try_join;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;

use rusty_processing::processing::{ProcessContextBuilder, processor, ProcessType};
use rusty_processing::stream_io::async_read_to_stream;

#[derive(Parser, Debug)]
struct Args {
    #[arg(
        short = 'i',
        long,
        value_parser = parse_file_path
    )]
    input: path::PathBuf,

    #[arg(
        short = 'o',
        long,
        value_parser = parse_directory_path
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

fn parse_path(path_str: &str) -> Result<path::PathBuf, String> {
    let input_path = path::PathBuf::from(path_str.to_string());
    if input_path.exists() {
        Ok(input_path)
    } else {
        Err(format!("Path {} not found", path_str))
    }
}

fn parse_file_path(path_str: &str) -> Result<path::PathBuf, String> {
    let path = parse_path(path_str)?;
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("path {} is not a file", path_str))
    }
}

fn parse_directory_path(path_str: &str) -> Result<path::PathBuf, String> {
    let path = parse_path(path_str)?;
    if path.is_dir() {
        Ok(path)
    } else {
        Err(format!("path {} is not a directory", path_str))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let types = if args.all {
        vec![ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf]
    } else {
        args.types
    };

    let file = Box::new(tokio::fs::File::open(&args.input).await?);
    let (stream, reading) = async_read_to_stream(file)?;

    let (output_sink, output_stream) = tokio::sync::mpsc::channel(100);
    let mut output_stream = Box::new(ReceiverStream::new(output_stream));

    let ctx = ProcessContextBuilder::new(
        args.mimetype,
        types,
        output_sink
    ).build();

    let processing = processor().process(ctx, stream);
    let output_handling = async {
        while let Some(output) = output_stream.next().await {
            match output {
                Ok(output) => println!("{:?}", output),
                Err(err) => println!("Error: {}", err),
            }
        }
        anyhow::Ok(())
    };

    try_join!(reading, processing, output_handling)?;
    Ok(())
}
