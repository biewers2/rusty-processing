use std::{fs, path};

use serde::Deserialize;

use common::assertions::{assert_identical, assert_identical_metadata};
use processing::processing::{ProcessContextBuilder, processor, ProcessOutput, ProcessOutputData, ProcessState, ProcessType};

use crate::common::assertions::assert_identical_text;

mod common;

#[derive(Debug, Deserialize)]
struct TestCase {
    mimetype: String,
    files: Vec<String>,
}

const REGRESSION_TEST_CASES_PATH: &str = "tests/regression-test-cases.json";

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

async fn process(mimetype: String, path_str: impl AsRef<str>) -> anyhow::Result<()> {
    let (output_sink, mut outputs) = tokio::sync::mpsc::channel(100);
    let ctx = ProcessContextBuilder::new(
        mimetype,
        vec![ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf],
        output_sink,
    ).build();

    let path = path::PathBuf::from(path_str.as_ref());
    let processing = tokio::spawn(processor().process(ctx, path));

    while let Some(output) = outputs.recv().await {
        match output? {
            ProcessOutput::Processed(state, data) => {
                assert_processed_output(expected_dir(&path_str, None), state, data)
            },
            ProcessOutput::Embedded(state, data, _) => {
                assert_embedded_output(expected_dir(&path_str, Some(&data.checksum)), state, data)
            }
        }
    }

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

fn expected_dir(path: impl AsRef<str>, checksum: Option<&str>) -> path::PathBuf {
    let path_str = format!("{}-expected", path.as_ref());
    let path = path::PathBuf::from(path_str);
    match checksum {
        Some(checksum) => path.join(checksum),
        None => path
    }
}