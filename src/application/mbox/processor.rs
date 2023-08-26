use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use mail_parser::mailbox::mbox::MessageIterator;
use threadpool::ThreadPool;

use crate::common::error::ProcessError;
use crate::processing::context::Context;
use crate::processing::process::Process;
use crate::processing::processor::processor;

pub struct MboxProcessor {
    pub(super) context: Context,
}

impl MboxProcessor {
    pub fn new(context: Context) -> Self {
        Self { context }
    }

    pub fn process<T: Read>(&self, message_iter: MessageIterator<BufReader<T>>) {
        let thread_pool = ThreadPool::new(100);

        for message_result in message_iter {
            match message_result {
                Ok(message) => {
                    let context = self.context.clone();
                    thread_pool.execute(move || {
                        processor().process_raw(
                            message.contents(),
                            context.output_dir.clone(),
                            "message/rfc822".to_string(),
                            context.types.clone(),
                            move |result| context.send_result(result),
                        );
                    });
                }
                Err(_) => self.context.send_result(Err(ProcessError::unexpected(
                    &self.context,
                    "failed to parse message from mbox",
                ))),
            }
        }

        thread_pool.join();
    }
}

impl Process for MboxProcessor {
    fn handle_file(&self, source_file: &PathBuf) {
        let result = File::open(source_file)
            .map(|file| MessageIterator::new(BufReader::new(file)))
            .map(|message_iter| self.process(message_iter))
            .map_err(|err| ProcessError::io(&self.context, err));

        if result.is_err() {
            self.context.send_result(result);
        }
    }

    fn handle_raw(&self, raw: &[u8]) {
        let message_iter = MessageIterator::new(BufReader::new(raw));
        self.process(message_iter);
    }
}
