use std::io::{BufReader, Read};

use anyhow::anyhow;
use mail_parser::mailbox::mbox::{Message, MessageIterator};
use serde::{Deserialize, Serialize};

use crate::common::workspace::Workspace;
use crate::process::{Process, ProcessContext, ProcessOutput, ProcessOutputType};

/// MboxProcessor is responsible for process mbox files.
///
/// Internally it uses the `mail_parser` crate to parse the mbox file.
/// The processor only writes out embedded messages and doesn't produce any processed output.
///
#[derive(Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct MboxProcessor;

impl MboxProcessor {
    /// Processes messages from an iterator.
    ///
    fn write_messages<T: Read>(&self, message_iter: MessageIterator<BufReader<T>>, context: ProcessContext) {
        for message_res in message_iter {
            let message_res = message_res.map_err(|err| anyhow!("failed to parse message from mbox: {:?}", err));
            match message_res {
                Ok(message) => {
                    let result = self.write_message(message, &context);
                    context.add_result(result);
                }
                Err(e) => context.add_result(Err(e)),
            }
        }
    }

    /// Writes a message to the output directory.
    ///
    fn write_message(&self, message: Message, context: &ProcessContext) -> anyhow::Result<ProcessOutput> {
        let mimetype = "message/rfc822";
        let Workspace { original_path, dupe_id, .. } = Workspace::new(
            &message.contents(),
            &context.output_dir,
            mimetype,
            &vec![],
        )?;

        Ok(ProcessOutput {
            path: original_path,
            output_type: ProcessOutputType::Embedded,
            mimetype: mimetype.to_string(),
            dupe_id,
        })
    }
}

impl Process for MboxProcessor {
    fn process(&self, content: Box<dyn Read + Send + Sync>, context: ProcessContext) {
        let reader = BufReader::new(content);
        self.write_messages(MessageIterator::new(reader), context);
    }
}

#[cfg(test)]
mod tests {
    use std::{path, thread};
    use std::fs::File;
    use std::sync::mpsc::Receiver;

    use super::*;

    fn processor_with_context() -> anyhow::Result<(MboxProcessor, ProcessContext, Receiver<anyhow::Result<ProcessOutput>>)> {
        let output_dir = tempfile::tempdir()?.into_path();
        let (context, rx) = ProcessContext::new(
            output_dir,
            "application/mbox",
            vec![],
        );
        Ok((MboxProcessor::default(), context, rx))
    }

    fn sort_embedded_outputs(outputs: &mut Vec<ProcessOutput>) {
        outputs.sort_by(|a, b| {
            match (&a.output_type, &b.output_type) {
                (ProcessOutputType::Embedded, ProcessOutputType::Embedded) => a.dupe_id.cmp(&b.dupe_id),
                _ => panic!("expected embedded output"),
            }
        });
    }

    #[test]
    fn test_process() -> anyhow::Result<()> {
        let path = path::PathBuf::from("resources/mbox/ubuntu-no-small.mbox");
        let (processor, context, mut rx) = processor_with_context()?;

        let outputs = thread::scope(move |scope| {
            let handle = thread::spawn(move || {
                let file = Box::new(File::open(path).unwrap());
                processor.process(file, context);
            });

            let mut outputs = vec![];
            while let Ok(output) = rx.recv() {
                outputs.push(output?)
            }

            // Sort to make the test deterministic
            sort_embedded_outputs(&mut outputs);
            anyhow::Ok(outputs)
        })?;

        assert_eq!(outputs.len(), 2);

        let output = & outputs[0];
        assert_eq!(output.output_type, ProcessOutputType::Embedded);
        assert_eq!(output.mimetype, "message/rfc822");
        assert_eq!(output.dupe_id, "4d338bc9f95d450a9372caa2fe0dfc97");

        let output = & outputs[1];
        assert_eq!(output.output_type, ProcessOutputType::Embedded);
        assert_eq!(output.mimetype, "message/rfc822");
        assert_eq!(output.dupe_id, "5e574a8f0d36b8805722b4e5ef3b7fd9");

        Ok(())
    }

    #[test]
    fn test_process_large_file() -> anyhow::Result<()> {
        let path = path::PathBuf::from("resources/mbox/ubuntu-no.mbox");
        let (processor, context, mut rx) = processor_with_context()?;

        let outputs = thread::scope(|scope| {
            scope.spawn(|| {
                let file = Box::new(File::open(path).unwrap());
                processor.process(file, context);
            });

            let mut outputs = vec![];
            while let Ok(output) = rx.recv() {
                outputs.push(output?)
            }

            // Sort to make the test deterministic
            sort_embedded_outputs(&mut outputs);
            anyhow::Ok(outputs)
        })?;

        assert_eq!(outputs.len(), 344);
        for output in outputs {
            assert_eq!(output.output_type, ProcessOutputType::Embedded);
            assert_eq!(output.mimetype, "message/rfc822");
        }

        Ok(())
    }
}
