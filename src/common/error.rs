use std::{fmt, io};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub type ProcessResult<T> = Result<T, ProcessError>;

#[derive(Debug)]
pub enum ProcessError {
  Unexpected(String),
  Duplicate,
  Trace(Box<ProcessError>, String),
  Io(io::Error),
}

impl ProcessError {
  pub fn from(message: &str) -> Self {
    Self::Unexpected(message.to_string())
  }

  pub fn from_io(err: io::Error, message: &str) -> Self {
    Self::Trace(Box::new(ProcessError::Io(err)), message.to_string())
  }
}

impl Display for ProcessError {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      ProcessError::Unexpected(msg) => writeln!(f, "{}", msg),
      ProcessError::Duplicate => writeln!(f, "Duplicate"),
      ProcessError::Trace(err, msg) => writeln!(f, "{}: {}", msg, err),
      ProcessError::Io(err) => Display::fmt(&err, f)
    }
  }
}

impl Error for ProcessError {
  fn cause(&self) -> Option<&dyn Error> {
    match self {
      ProcessError::Unexpected(_) | ProcessError::Duplicate => None,
      ProcessError::Trace(err, _) => Some(err),
      ProcessError::Io(err) => Some(err)
    }
  }
}
