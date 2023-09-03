use std::borrow::Cow;
use std::thread;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread::Scope;
use anyhow::anyhow;

use mail_parser::mailbox::mbox::{Message, MessageIterator};

use crate::common::workspace::Workspace;
use crate::processing::context::Context;
use crate::processing::output::{OutputInfo, Output};
use crate::processing::process::Process;

pub struct MboxProcessor {
    pub(super) context: Context,
}

impl MboxProcessor {
    pub fn new(context: Context) -> Self {
        Self { context }
    }

    fn process<T: Read>(&self, message_iter: MessageIterator<BufReader<T>>) {
        thread::scope(|s| self.proccess_in_scope(message_iter, s));
    }

    fn proccess_in_scope<'a, T: Read>(&'a self, message_iter: MessageIterator<BufReader<T>>, scope: &'a Scope<'a, '_>) {
        for message_result in message_iter {
            scope.spawn(||
                self.context.send_result(
                    message_result
                        .map_err(|err| anyhow!("failed to parse message from mbox: {:?}", err))
                        .and_then(|message| self.prepare_message_output(&message))
                )
            );
        }
    }

    fn prepare_message_output(&self, message: &Message) -> anyhow::Result<Output> {
        let context = Context {
            output_dir: self.context.output_dir.clone(),
            types: vec![],
            mimetype: "message/rfc822".to_string(),
            result_tx: Mutex::new(None),
        };

        let wkspace = Workspace::new(&context, &message.contents())?;
        Ok(Output::Embedded(OutputInfo {
            path: wkspace.original_path,
            mimetype: context.mimetype,
            dupe_id: wkspace.dupe_id,
        }))
    }
}

impl Process for MboxProcessor {
    fn handle_file(&self, source_file: &PathBuf) {
        let result = File::open(source_file)
            .map_err(|err| anyhow!("failed to open mbox file: {:?}", err))
            .map(|file| MessageIterator::new(BufReader::new(file)))
            .map(|message_iter| self.process(message_iter));

        if let Err(err) = result {
            self.context.send_result(Err(err));
        }
    }

    fn handle_raw(&self, raw: Cow<[u8]>) {
        let message_iter = MessageIterator::new(BufReader::new(raw.as_ref()));
        self.process(message_iter);
    }
}
