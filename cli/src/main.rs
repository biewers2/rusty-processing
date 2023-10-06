use std::path;

use clap::Parser;

use processing::io::async_read_to_stream;
use processing::process_rusty_stream;
use processing::processing::ProcessType;

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
    let args = Args::parse();
    let types = if args.all {
        vec![ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf]
    } else {
        args.types
    };

    let file = Box::new(tokio::fs::File::open(&args.input).await?);
    let (stream, reading) = async_read_to_stream(file)?;
    let reading = tokio::spawn(reading);

    let mut archive_file = process_rusty_stream(stream, args.mimetype, types, true).await?;
    reading.await??;

    let mut output_file = tokio::fs::File::create(args.output).await?;
    tokio::io::copy(&mut archive_file, &mut output_file).await?;

    Ok(())
}