use std::path;

use clap::Parser;

use processing::process;
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
    simple_logger::init_with_level(log::Level::Info)?;

    let args = Args::parse();
    let types = if args.all {
        vec![ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf]
    } else {
        args.types
    };

    let mut resulting_file = process(args.input, args.mimetype, types, true).await?;
    let mut output_file = tokio::fs::File::create(args.output).await?;
    tokio::io::copy(&mut resulting_file, &mut output_file).await?;

    Ok(())
}
