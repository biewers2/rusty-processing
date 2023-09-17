use std::fs::File;
use std::{path, thread};

use anyhow::anyhow;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::application::mbox::processor::MboxProcessor;
use crate::process::{ProcessContext, ProcessType};
use crate::process::process::Process;
use crate::process::process_output::ProcessOutput;

lazy_static! {
    static ref PROCESSOR: Processor = Processor;
}

/// Returns a reference to the global processor instance.
///
pub fn processor() -> &'static Processor {
    &PROCESSOR
}

/// Structure defining the core processor.
///
/// The processor is the core processor of the library and is responsible for
/// determining the correct processor to use for a given MIME type, and then
/// delegating to that processor.
///
#[derive(Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct Processor;

impl Processor {
    /// Processes a file.
    ///
    /// This method will determine the correct processor to use for the given
    /// MIME type, and then delegate to that processor.
    ///
    /// # Arguments
    ///
    /// * `source_file` - The path to the file to process.
    /// * `output_dir` - The path to the directory to write output files to.
    /// * `mimetype` - The MIME type of the file to process.
    /// * `types` - The types of output to generate.
    ///
    pub fn process<F>(
        &self,
        source_file: path::PathBuf,
        output_dir: path::PathBuf,
        mimetype: String,
        types: Vec<ProcessType>,
        handle_result: &mut F,
    ) -> anyhow::Result<()>
        where F: FnMut(anyhow::Result<ProcessOutput>) + Send + Sync,
    {
        let (context, rx) = ProcessContext::new(
            output_dir,
            mimetype,
            types,
        );

        thread::scope(|scope| {
            let handle = scope.spawn(|| {
                let processor = Self::processor_for_mimetype(&context.mimetype)?;
                let file = Box::new(File::open(source_file)?);
                processor.process(file, context);
                anyhow::Ok(())
            });

            while let Ok(res) = rx.recv() {
                handle_result(res);
            }

            match handle.join() {
                Ok(_) => Ok(()),
                Err(e) => Err(anyhow!("Processing failed: {:?}", e)),
            }
        })
    }

    fn processor_for_mimetype(mimetype: &str) -> anyhow::Result<Box<dyn Process>> {
        match mimetype {
            "application/mbox" => Ok(Box::new(MboxProcessor)),
            // "message/rfc822" => Ok(Box::new(Rfc822Processor::default())),
            _ => Err(anyhow!("Unsupported MIME type: {}", mimetype)),
        }
    }
}
