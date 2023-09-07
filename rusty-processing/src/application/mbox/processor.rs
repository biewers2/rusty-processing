use std::borrow::Cow;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;

use anyhow::anyhow;
use mail_parser::mailbox::mbox::{Message, MessageIterator};

use crate::common::workspace::Workspace;
use crate::processing::context::Context;
use crate::processing::output::{Output, OutputInfo};
use crate::processing::process::Process;

/// MboxProcessor is responsible for processing mbox files.
///
/// Internally it uses the `mail_parser` crate to parse the mbox file.
/// The processor only writes out embedded messages and doesn't produce any processed output.
///
pub struct MboxProcessor {
    pub(super) context: Context,
}

impl MboxProcessor {
    /// Creates a new MboxProcessor with the given context.
    ///
    pub fn new(context: Context) -> Self {
        Self { context }
    }

    /// Processes messages from an iterator.
    ///
    fn process<T: Read>(&self, message_iter: MessageIterator<BufReader<T>>) {
        thread::scope(|s| {
            for message_result in message_iter {
                s.spawn(|| {
                    self.context.send_result(
                        message_result
                            .map_err(|err| anyhow!("failed to parse message from mbox: {:?}", err))
                            .and_then(|message| self.write_message(&message)),
                    )
                });
            }
        });
    }

    /// Writes a message to the output directory.
    ///
    fn write_message(&self, message: &Message) -> anyhow::Result<Output> {
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

#[cfg(test)]
mod tests {
    use std::path;
    use std::sync::mpsc::Sender;

    use crate::processing::context::Context;

    use super::*;

    fn processor(tx: Sender<anyhow::Result<Output>>) -> anyhow::Result<MboxProcessor> {
        let output_dir = tempfile::tempdir()?.into_path();
        Ok(MboxProcessor::new(Context {
            output_dir,
            mimetype: "application/mbox".to_string(),
            types: vec![],
            result_tx: std::sync::Mutex::new(Some(tx)),
        }))
    }

    #[test]
    fn test_process_handle_file() -> anyhow::Result<()> {
        let path = path::PathBuf::from("resources/mbox/ubuntu-no-small.mbox");

        let (tx, rx) = std::sync::mpsc::channel();
        let processor = processor(tx)?;

        let mut outputs = thread::scope(move |s| {
            s.spawn(move || processor.handle_file(&path));
            rx.into_iter()
                .map(|result| result.unwrap())
                .collect::<Vec<Output>>()
        });
        // Sort to make the test deterministic.
        outputs.sort_by(|a, b| {
            match (a, b) {
                (Output::Embedded(a), Output::Embedded(b)) => a.dupe_id.cmp(&b.dupe_id),
                _ => panic!("expected embedded output"),
            }
        });

        assert_eq!(outputs.len(), 2);
        if let Output::Embedded(info) = &outputs[0] {
            assert_eq!(info.mimetype, "message/rfc822");
            assert_eq!(info.dupe_id, "4d338bc9f95d450a9372caa2fe0dfc97");
        } else {
            panic!("expected embedded output");
        }
        if let Output::Embedded(info) = &outputs[1] {
            assert_eq!(info.mimetype, "message/rfc822");
            assert_eq!(info.dupe_id, "5e574a8f0d36b8805722b4e5ef3b7fd9");
        } else {
            panic!("expected embedded output");
        }

        Ok(())
    }
}
