use anyhow::anyhow;
use std::borrow::Cow;
use std::sync::{mpsc, Mutex};
use std::{path, thread};

use lazy_static::lazy_static;

use crate::application::mbox::processor::MboxProcessor;
use crate::common::output_type::ProcessType;
use crate::message::rfc822::processor::Rfc822Processor;
use crate::processing::context::Context;
use crate::processing::output::Output;
use crate::processing::process::Process;

lazy_static! {
    static ref PROCESSOR: Processor = Processor::default();
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
#[derive(Default)]
pub struct Processor {}

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
    pub fn process_file<F>(
        &self,
        source_file: path::PathBuf,
        output_dir: path::PathBuf,
        mimetype: String,
        types: Vec<ProcessType>,
        handle_result: F,
    ) where
        F: Fn(anyhow::Result<Output>) -> (),
        F: Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let context = Context {
            output_dir,
            mimetype,
            types,
            result_tx: Mutex::new(Some(tx)),
        };

        thread::spawn(move || loop {
            match rx.recv() {
                Ok(res) => handle_result(res),
                Err(_) => break,
            }
        });

        self.process_mime(context, |processor| processor.handle_file(&source_file))
    }

    /// Processes a raw byte array.
    ///
    /// This method will determine the correct processor to use for the given
    /// MIME type, and then delegate to that processor.
    ///
    /// # Arguments
    ///
    /// * `raw` - The raw byte array to process.
    /// * `output_dir` - The path to the directory to write output files to.
    /// * `mimetype` - The MIME type of the file to process.
    /// * `types` - The types of output to generate.
    ///
    pub fn process_raw<F>(
        &self,
        raw: Cow<[u8]>,
        output_dir: path::PathBuf,
        mimetype: String,
        types: Vec<ProcessType>,
        handle_result: F,
    ) where
        F: Fn(anyhow::Result<Output>) -> (),
    {
        let (tx, rx) = mpsc::channel();
        let context = Context {
            output_dir,
            mimetype,
            types,
            result_tx: Mutex::new(Some(tx)),
        };

        thread::scope(|s| {
            s.spawn(|| {
                self.process_mime(context, |processor| processor.handle_raw(raw.clone()));
            });

            while let Ok(res) = rx.recv() {
                handle_result(res);
            }
        });
    }

    /// Delegates processing to a specific processor based on the MIME type.
    ///
    fn process_mime<F>(&self, context: Context, block: F)
    where
        F: Fn(Box<dyn Process>),
    {
        match context.mimetype.as_str() {
            "application/mbox" => block(Box::new(MboxProcessor::new(context))),
            "message/rfc822" => block(Box::new(Rfc822Processor::new(context))),
            _ => context.send_result(Err(anyhow!("Unsupported MIME type: {}", context.mimetype))),
        }
    }
}
