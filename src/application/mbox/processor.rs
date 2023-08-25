use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

use lazy_static::lazy_static;
use mail_parser::mailbox::mbox::{Message, MessageIterator};
use threadpool::ThreadPool;

use crate::common::error::{ProcessError, ProcessResult};
use crate::common::output::OutputType;
use crate::message::rfc822::processor::rfc822_processor;
use crate::processing::process::{Process, ProcessService};

lazy_static! {
  static ref MBOX_PROCESSOR: ProcessService = Box::<MboxProcessor>::default();
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
    output_dir: &PathBuf,
    types: &Vec<OutputType>,
  ) -> ProcessResult<()>
    where T: Read,
  {
    let thread_pool = ThreadPool::new(100);
    let (tx, rx): (Sender<ProcessResult<()>>, Receiver<ProcessResult<()>>) = mpsc::channel();

    let mut num_threads = 0;
    for message_result in message_iter {
      let message = message_result.map_err(|_| ProcessError::from("failed to parse message from mbox"))?;

      let tx = tx.clone();
      let output_dir = output_dir.clone();
      let types = types.clone();

      thread_pool.execute(move ||
        tx.send(rfc822_processor().handle_raw(message.contents(), &output_dir, &types)).unwrap()
      );

      num_threads += 1;
    }

    let mut success_count = 0;
    let mut failure_count = 0;
    for _ in 0..num_threads {
      match rx.recv().unwrap() {
        Ok(_) => success_count += 1,
        Err(_) => failure_count += 1,
      }

      if (success_count + failure_count) % 100 == 0 {
        println!("{} successful, {} failed", success_count, failure_count);
      }
    }

    thread_pool.join();
    Ok(())
  }
}

impl Process for MboxProcessor {
  fn handle_file(
    &self,
    source_file: &PathBuf,
    output_dir: &PathBuf,
    types: &Vec<OutputType>,
  ) -> ProcessResult<()> {
    let file= File::open(source_file).map_err(ProcessError::Io)?;
    let message_iter = MessageIterator::new(BufReader::new(file));
    self.process(message_iter, output_dir, types)
  }

  fn handle_raw(
    &self,
    raw: &[u8],
    output_dir: &PathBuf,
    types: &Vec<OutputType>,
  ) -> ProcessResult<()> {
    let message_iter = MessageIterator::new(BufReader::new(raw));
    self.process(message_iter, output_dir, types)
  }
}
