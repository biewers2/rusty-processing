use std::path;

use clap::Parser;

use rusty_processing::common::error::ProcessResult;
use rusty_processing::common::output::OutputType;
use rusty_processing::processing::processor::processor;

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
    num_args = 1..,
    value_delimiter = ' ',
  )]
  types: Vec<OutputType>
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

fn main() -> ProcessResult<()> {
  let args = Args::parse();
  let processor = processor().lock().unwrap();
  let types =
    if args.types.is_empty() {
      None
    } else {
      Some(&args.types)
    };

  processor
    .process_file(
      &args.input,
      &args.output,
      &args.mimetype,
      types,
    )
}
