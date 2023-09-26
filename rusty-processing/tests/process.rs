use std::collections::HashSet;

use rusty_processing::processing::{processor, ProcessOutputData, ProcessOutputForm, ProcessType};

#[test]
fn test_mbox_file() -> anyhow::Result<()> {
    let expecteds = HashSet::from([
        ProcessOutputData {
            output_type: ProcessOutputForm::Embedded,
            path: "4d338bc9f95d450a9372caa2fe0dfc97/original.eml".into(),
            mimetype: "message/rfc822".into(),
            dupe_id: "4d338bc9f95d450a9372caa2fe0dfc97".into(),
        },
        ProcessOutputData {
            output_type: ProcessOutputForm::Embedded,
            path: "5e574a8f0d36b8805722b4e5ef3b7fd9/original.eml".into(),
            mimetype: "message/rfc822".into(),
            dupe_id: "5e574a8f0d36b8805722b4e5ef3b7fd9".into(),
        },
    ]);

    let source_path = "resources/mbox/ubuntu-no-small.mbox".into();
    let temp_dir = tempfile::tempdir()?.into_path();

    let mut actuals = HashSet::new();
    let results_ref = &mut actuals;
    processor().process(
        source_path,
        temp_dir,
        "application/mbox".to_string(),
        vec![ProcessType::Text, ProcessType::Metadata, ProcessType::Pdf],
        &mut move |result| {
            match result {
                Ok(output) => {
                    results_ref.insert(output);
                },
                Err(e) => {
                    eprintln!("Error processing file: {}", e);
                    panic!("Error processing file");
                },
            }
        },
    )?;

    assert_eq!(expecteds, actuals);
    Ok(())
}