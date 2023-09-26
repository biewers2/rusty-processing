use std::io::{BufReader, Read};

use anyhow::anyhow;
use async_trait::async_trait;
use mail_parser::mailbox::mbox::{Message, MessageIterator};
use serde::{Deserialize, Serialize};

use crate::common::{ByteStream, StreamReader};
use crate::common::workspace::Workspace;
use crate::processing::{Process, ProcessContext, ProcessOutput};

/// MboxProcessor is responsible for processing mbox files.
///
/// Internally it uses the `mail_parser` crate to parse the mbox file.
/// The processor only writes out embedded messages and doesn't produce any processed output.
///
#[derive(Debug, Default, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub struct MboxProcessor;

impl MboxProcessor {
    /// Processes messages from an iterator.
    ///
    async fn write_messages<T: Read>(&self, ctx: ProcessContext, message_iter: MessageIterator<BufReader<T>>) -> anyhow::Result<()> {
        for message_res in message_iter {
            let message = message_res.map_err(|err| anyhow!("failed to parse message from mbox: {:?}", err))?;
            let result = self.write_message(&ctx, message).await;
            ctx.add_output(result).await?;
        }
        Ok(())
    }

    /// Writes a message to the output directory.
    ///
    async fn write_message(&self, ctx: &ProcessContext, message: Message) -> anyhow::Result<ProcessOutput> {
        let mimetype = "message/rfc822";
        let Workspace { original_path, dupe_id, .. } = Workspace::new(
            message.contents(),
            mimetype,
            &[],
        )?;

        let ctx = ctx.new_clone(mimetype.to_string());

        Ok(ProcessOutput::embedded(&ctx, original_path, mimetype.to_string(), dupe_id))
    }
}

#[async_trait]
impl Process for MboxProcessor {
    async fn process(&self, ctx: ProcessContext, content: ByteStream) -> anyhow::Result<()> {
        let content = StreamReader::new(Box::new(content));
        let reader = BufReader::new(content);
        self.write_messages(ctx, MessageIterator::new(reader)).await
    }
}

#[cfg(test)]
mod tests {
    use std::{path, thread};
    use std::fs::File;
    use std::sync::mpsc::Receiver;

    use super::*;

    fn processor_with_context() -> anyhow::Result<(MboxProcessor, ProcessContext, Receiver<anyhow::Result<ProcessOutputContext>>)> {
        let (context, rx) = ProcessContext::new(
            "application/mbox",
            vec![],
        );
        Ok((MboxProcessor::default(), context, rx))
    }

    fn sort_embedded_outputs(outputs: &mut Vec<ProcessOutputContext>) {
        outputs.sort_by(|a, b| {
            match (&a.output_type, &b.output_type) {
                (ProcessOutputForm::Embedded, ProcessOutputForm::Embedded) => a.dupe_id.cmp(&b.dupe_id),
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

            // Sort to make the services deterministic
            sort_embedded_outputs(&mut outputs);
            anyhow::Ok(outputs)
        })?;

        assert_eq!(outputs.len(), 2);

        let output = & outputs[0];
        assert_eq!(output.output_type, ProcessOutputForm::Embedded);
        assert_eq!(output.mimetype, "message/rfc822");
        assert_eq!(output.dupe_id, "4d338bc9f95d450a9372caa2fe0dfc97");

        let output = & outputs[1];
        assert_eq!(output.output_type, ProcessOutputForm::Embedded);
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

            // Sort to make the services deterministic
            sort_embedded_outputs(&mut outputs);
            anyhow::Ok(outputs)
        })?;

        assert_eq!(outputs.len(), 344);
        for output in outputs {
            assert_eq!(output.output_type, ProcessOutputForm::Embedded);
            assert_eq!(output.mimetype, "message/rfc822");
        }

        Ok(())
    }
}
