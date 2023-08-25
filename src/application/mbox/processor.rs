use std::fs::File;
use std::io::{BufReader, Read};
use std::{path, thread};
use std::sync::Mutex;

use lazy_static::lazy_static;
use mail_parser::mailbox::mbox::MessageIterator;

use crate::common::error::{ProcessError, ProcessResult};
use crate::common::output::OutputType;
use crate::message::rfc822::processor::rfc822_processor;
use crate::processing::process::{Process, ProcessService};

lazy_static! {
  static ref MBOX_PROCESSOR: ProcessService = Mutex::new(Box::<MboxProcessor>::default());
}

pub fn mbox_processor() -> &'static ProcessService {
  &MBOX_PROCESSOR
}

#[derive(Default)]
pub struct MboxProcessor {}

impl MboxProcessor {
  pub fn process<T>(
    &self,
    message_iter: MessageIterator<BufReader<T>>,
    output_dir: path::PathBuf,
    types: Vec<OutputType>,
  ) -> ProcessResult<()>
    where T: Read
  {
    let pool = threadpool::ThreadPool::new(20);
    for msg_res in message_iter {
      pool.execute(||
        msg_res
          .map_err(|_| ProcessError::from("failed to parse message from mbox"))
          .and_then(|msg| {
            rfc822_processor()
              .lock()
              .unwrap()
              .handle_raw(msg.contents(), &output_dir, &types)
          })
          .unwrap_or_else(|err| println!("Failed to process message from mbox: {}", err))
      );
    }
    Ok(())
  }
}

impl Process for MboxProcessor {
  fn handle_file(
    &self,
    source_file: &path::PathBuf,
    output_dir: &path::PathBuf,
    types: &Vec<OutputType>,
  ) -> ProcessResult<()> {
    let file= File::open(source_file).map_err(ProcessError::Io)?;
    let message_iter = MessageIterator::new(BufReader::new(file));
    self.process(message_iter, output_dir, types)
  }

  fn handle_raw(
    &self,
    raw: &[u8],
    output_dir: &path::PathBuf,
    types: &Vec<OutputType>,
  ) -> ProcessResult<()> {
    let message_iter = MessageIterator::new(BufReader::new(raw));
    self.process(message_iter, output_dir, types)
  }
}
