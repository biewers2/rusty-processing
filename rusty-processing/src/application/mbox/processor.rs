use std::io::{BufReader, Read};
use std::ops::Deref;

use anyhow::anyhow;
use async_trait::async_trait;
use mail_parser::mailbox::mbox::{Message, MessageIterator};
use serde::{Deserialize, Serialize};
use rusty_processing_identify::identifier;

use crate::common::{ByteStream};
use crate::common::workspace::Workspace;
use crate::processing::{Process, ProcessContext, ProcessOutput};
use crate::stream_io::stream_to_read;

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
        let Workspace { original_path: original_file, .. } = Workspace::new(message.contents(), &[])?;
        let ctx = ctx.new_clone(mimetype.to_string());
        let dedupe_id = identifier(mimetype).identify(message.contents());

        Ok(ProcessOutput::embedded(&ctx, "mbox-message.eml", original_file, mimetype, dedupe_id))
    }
}

#[async_trait]
impl Process for MboxProcessor {
    async fn process(&self, ctx: ProcessContext, content: ByteStream) -> anyhow::Result<()> {
        let reader = BufReader::new(stream_to_read(content));
        self.write_messages(ctx, MessageIterator::new(reader)).await
    }
}

#[cfg(test)]
mod tests {
    use std::path;

    use tokio::sync::mpsc::Receiver;
    use tokio::task::JoinHandle;

    use crate::processing::ProcessContextBuilder;
    use crate::test_util::byte_stream_from_fs;

    use super::*;

    type ProcessFuture = JoinHandle<anyhow::Result<()>>;
    type OutputReceiver = Receiver<anyhow::Result<ProcessOutput>>;

    fn processor_with_context() -> anyhow::Result<(MboxProcessor, ProcessContext, Receiver<anyhow::Result<ProcessOutput>>)> {
        let (output_sink, outputs) = tokio::sync::mpsc::channel(10);
        let ctx = ProcessContextBuilder::new("application/mbox", vec![], output_sink).build();
        Ok((MboxProcessor, ctx, outputs))
    }

    fn process(path: path::PathBuf) -> anyhow::Result<(ProcessFuture, OutputReceiver)> {
        let (processor, ctx, output_rx) = processor_with_context()?;
        let proc_fut = tokio::spawn(async move {
            let stream = byte_stream_from_fs(path).await?;
            processor.process(ctx, stream).await?;
            anyhow::Ok(())
        });
        Ok((proc_fut, output_rx))
    }

    #[tokio::test]
    async fn test_process() -> anyhow::Result<()> {
        let path = path::PathBuf::from("resources/mbox/ubuntu-no-small.mbox");
        let (proc_fut, mut output_rx) = process(path)?;

        let mut outputs = vec![];
        while let Some(output) = output_rx.recv().await {
            match output? {
                ProcessOutput::Processed(_, _) => panic!("Expected embedded output"),
                ProcessOutput::Embedded(state, data, _) => outputs.push((state, data))
            }
        }
        proc_fut.await??;

        // Sort to make the test deterministic
        outputs.sort_by(|o0, o1| o0.1.dedupe_id.cmp(&o1.1.dedupe_id));

        assert_eq!(outputs.len(), 2);

        let (state, ctx) = &outputs[0];
        assert_eq!(ctx.mimetype, "message/rfc822");
        assert_eq!(ctx.dedupe_id, "4d338bc9f95d450a9372caa2fe0dfc97");
        assert_eq!(state.id_chain, Vec::<String>::new());

        let (state, ctx) = &outputs[1];
        assert_eq!(ctx.mimetype, "message/rfc822");
        assert_eq!(ctx.dedupe_id, "5e574a8f0d36b8805722b4e5ef3b7fd9");
        assert_eq!(state.id_chain, Vec::<String>::new());

        Ok(())
    }

    #[tokio::test]
    async fn test_process_large_file() -> anyhow::Result<()> {
        let path = path::PathBuf::from("resources/mbox/ubuntu-no.mbox");
        let (proc_fut, mut output_rx) = process(path)?;

        let mut output_count = 0;
        while let Some(output) = output_rx.recv().await {
            match output? {
                ProcessOutput::Processed(_, _) => panic!("Expected embedded output"),
                ProcessOutput::Embedded(_, data, _) => {
                    output_count += 1;
                    assert_eq!(data.mimetype, "message/rfc822");
                }
            }
        }
        proc_fut.await??;

        assert_eq!(output_count, 344);
        Ok(())
    }
}
