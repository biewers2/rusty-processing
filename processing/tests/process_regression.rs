mod common;

use std::{fs, path};
use std::fs::File;
use serde::Deserialize;
use processing::io::read_to_stream;
use processing::processing::{ProcessContextBuilder, processor, ProcessOutput, ProcessOutputData, ProcessState, ProcessType};
use common::assertions::{assert_identical, assert_identical_metadata};
use crate::common::assertions::assert_identical_text;

#[derive(Debug, Deserialize)]
struct TestCase {
    mimetype: String,
    files: Vec<String>,
}

const REGRESSION_TEST_CASES_PATH: &str = "resources/regression-test-cases.json";

#[tokio::test]
async fn test_process_regression() -> anyhow::Result<()> {
    let json_str = fs::read_to_string(REGRESSION_TEST_CASES_PATH)?;
    let test_cases: Vec<TestCase> = serde_json::from_str(&json_str)?;

    for case in test_cases {
        for file_path_str in case.files {
            process(case.mimetype.clone(), file_path_str).await?;
        }
    }

    Ok(())
}

async fn process(mimetype: String, path: String) -> anyhow::Result<()> {
    let file = Box::new(File::open(&path)?);
    let (stream, reading) = read_to_stream(file)?;
    let reading = tokio::spawn(reading);

    let (output_sink, mut outputs) = tokio::sync::mpsc::channel(100);
    let ctx = ProcessContextBuilder::new(
        mimetype,
        vec![ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf],
        output_sink,
    ).build();

    let processing = tokio::spawn(processor().process(ctx, stream));

    while let Some(output) = outputs.recv().await {
        match output? {
            ProcessOutput::Processed(state, data) => {
                assert_processed_output(expected_dir(&path, None), state, data)
            },
            ProcessOutput::Embedded(state, data, _) => {
                assert_embedded_output(expected_dir(&path, Some(&data.dedupe_id)), state, data)
            }
        }
    }

    reading.await??;
    processing.await??;
    Ok(())
}

fn assert_processed_output(expected_dir: path::PathBuf, _state: ProcessState, data: ProcessOutputData) {
    let name = data.name.as_str();
    let expected_path = expected_dir.join(name);

    match name {
        "extracted.txt" => assert_identical_text(expected_path, data.path),
        "metadata.json" => assert_identical_metadata(expected_path, data.path),
        "rendered.pdf" => (), // assert_identical(expected_path, data.path),
        _ => panic!("Unexpected file name: {:?}", name),
    };
}

fn assert_embedded_output(expected_dir: path::PathBuf, _state: ProcessState, data: ProcessOutputData) {
    let name = data.name.as_str();
    let expected_path = expected_dir.join(name);

    assert_identical(expected_path, data.path);
}

fn expected_dir(path: &str, dedupe_id: Option<&str>) -> path::PathBuf {
    let path_str = format!("{}-expected", path);
    let path = path::PathBuf::from(path_str);
    match dedupe_id {
        Some(dedupe_id) => path.join(dedupe_id),
        None => path
    }
}