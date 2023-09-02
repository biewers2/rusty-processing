use std::{path, thread};

use serde::{Deserialize, Serialize};
use temporal_sdk::ActContext;
use rusty_processing::common::output_type::ProcessType;

use rusty_processing::processing::output::{Output, OutputInfo};
use rusty_processing::processing::processor::processor;

#[derive(Deserialize, Debug)]
pub struct ProcessRustyFileInput {
    pub source_path: path::PathBuf,
    pub output_dir: path::PathBuf,
    pub mimetype: String,
}

#[derive(Serialize, Debug)]
pub struct TmpFailureOutput {
    message: String,
}

#[derive(Serialize, Debug)]
pub struct ProcessRustyFileOutput {
    pub processed: Vec<OutputInfo>,
    pub embedded: Vec<OutputInfo>,
    pub failures: Vec<TmpFailureOutput>,
}

pub async fn process_rusty_file(
    _ctx: ActContext,
    input: ProcessRustyFileInput,
) -> anyhow::Result<ProcessRustyFileOutput> {
    let mut processed = vec![];
    let mut embedded = vec![];
    let mut failures = vec![];

    thread::scope(|s| {
        let (tx, rx) = std::sync::mpsc::channel();
        let (embedded, failures) = (&mut embedded, &mut failures);
        let types = vec![ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf];

        s.spawn(|| {
            processor().process_file(
                input.source_path,
                input.output_dir,
                input.mimetype,
                types,
                move |result| tx.send(result).unwrap_or(()),
            );
        });

        while let Ok(result) = rx.recv() {
            match result {
                Ok(Output::Processed(output)) => processed.push(output),
                Ok(Output::Embedded(output)) => embedded.push(output),
                Err(err) => failures.push(TmpFailureOutput {
                    message: err.to_string(),
                }),
            }
        }
    });

    Ok(ProcessRustyFileOutput {
        processed,
        embedded,
        failures
    })
}
