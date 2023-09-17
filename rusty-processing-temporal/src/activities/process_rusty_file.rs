use std::{path, thread};

use serde::{Deserialize, Serialize};
use temporal_sdk::ActContext;

use rusty_processing::processing::{processor, ProcessOutput, ProcessType};

#[derive(Deserialize, Debug)]
pub struct ProcessRustyFileInput {
    pub source_path: path::PathBuf,
    pub output_dir: path::PathBuf,
    pub mimetype: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TmpFailureOutput {
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessRustyFileOutput {
    pub results: Vec<ProcessOutput>,
    pub failures: Vec<TmpFailureOutput>,
}

pub async fn process_rusty_file(
    _ctx: ActContext,
    input: ProcessRustyFileInput,
) -> anyhow::Result<ProcessRustyFileOutput> {
    let mut results = vec![];
    let mut failures = vec![];

    thread::scope(|s| {
        let (tx, rx) = std::sync::mpsc::channel();
        let (results, failures) = (&mut results, &mut failures);
        let types = vec![ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf];

        s.spawn(|| processor().process(
            input.source_path,
            input.output_dir,
            input.mimetype,
            types,
            &mut move |result| tx.send(result).unwrap_or(()),
        ));

        while let Ok(result) = rx.recv() {
            match result {
                Ok(output) => results.push(output),
                Err(err) => failures.push(TmpFailureOutput {
                    message: err.to_string(),
                }),
            }
        }
    });

    Ok(ProcessRustyFileOutput {
        results,
        failures,
    })
}
