use crate::processing::context::Context;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::{fmt, io};

/// A type alias for the result of a processing operation.
///
/// [`ProcessError`] is used as the error type.
///
pub type ProcessResult<T> = Result<T, ProcessError>;

/// An error type for processing operations.
///
/// This type is used as the error type for [`ProcessResult`].
/// It is highly recommended to use the associated builder methods to create instances of this type,
/// as the context needs to be copied in a particular way to maintain stability and performance.
///
#[derive(Debug)]
pub enum ProcessError {
    /// Indicates the file being processed is a duplicate of a file that was already processed.
    ///
    /// The first field is the ID (deduplication ID) of the file, and the second field is the MIME type of the
    /// file, if known.
    ///
    Duplicate(Context, String),

    /// Indicates that no processor is implemented for the given MIME type.
    ///
    /// The first field is the MIME type of the file.
    ///
    NoProcessor(Context),

    /// Used to wrap context around another processing error.
    ///
    /// The first field is the wrapped error, and the second field is a message describing the context.
    ///
    Trace(Box<ProcessError>, String),

    /// Used to encapsulate multiple processing errors.
    ///
    /// This could be used in the case of processing a set of embedded files, where some of the files
    /// fail to process but the failing context should still be maintained.
    Collection(Vec<Box<ProcessError>>),

    /// Used to wrap an [`io::Error`].
    ///
    Io(Context, io::Error),

    /// Used to indicate an unexpected error.
    ///
    /// The first field is a message describing the error.
    ///
    Unexpected(Context, String),
}

impl ProcessError {
    /// Creates a new [`ProcessError::Duplicate`] variant.
    ///
    pub fn duplicate(context: &Context, dupe_id: &str) -> Self {
        Self::Duplicate(Self::clean_context(context), dupe_id.to_string())
    }

    /// Creates a new [`ProcessError::NoProcessor`] variant.
    ///
    pub fn no_processor(context: &Context) -> Self {
        Self::NoProcessor(Self::clean_context(context))
    }

    /// Creates a new [`ProcessError::Unexpected`] variant.
    ///
    /// The first field is a message describing the error.
    ///
    pub fn unexpected(context: &Context, message: &str) -> Self {
        Self::Unexpected(Self::clean_context(context), message.to_string())
    }

    /// Creates a new [`ProcessError::Io`] variant.
    ///
    /// The first field is the I/O error.
    ///
    pub fn io(context: &Context, err: io::Error) -> Self {
        Self::Io(Self::clean_context(context), err)
    }

    /// Creates a new [`ProcessError::Trace`] variant.
    ///
    /// The first field is the wrapped error, and the second field is a message describing the context.
    ///
    pub fn from_io(context: &Context, err: io::Error, message: &str) -> Self {
        Self::Trace(Box::new(Self::io(context, err)), message.to_string())
    }

    /// Creates a new [`ProcessError::Collection`] variant.
    ///
    /// Wraps each error in the given vector in a [`Box`] and puts them into a [`ProcessError::Collection`].
    ///
    pub fn from_vec(errs: Vec<ProcessError>) -> Self {
        ProcessError::Collection(
            errs.into_iter()
                .map(|err| Box::new(err))
                .collect::<Vec<Box<ProcessError>>>(),
        )
    }

    fn clean_context(context: &Context) -> Context {
        Context {
            output_dir: context.output_dir.clone(),
            mimetype: context.mimetype.clone(),
            types: context.types.clone(),
            result_tx: None,
        }
    }
}

impl Display for ProcessError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ProcessError::Duplicate(context, dupe_id) => {
                writeln!(
                    f,
                    "Duplicate file found: {} ({})",
                    dupe_id, context.mimetype
                )
            }

            ProcessError::NoProcessor(context) => {
                writeln!(f, "No processor implemented yet for '{}'", context.mimetype)
            }

            ProcessError::Trace(err, msg) => writeln!(f, "{}: {}", msg, err),

            ProcessError::Collection(errs) => {
                for err in errs {
                    writeln!(f, "{}", err)?;
                }
                Ok(())
            }

            ProcessError::Io(_, err) => Display::fmt(&err, f),

            ProcessError::Unexpected(_, message) => writeln!(f, "Unexpected error: {}", message),
        }
    }
}

impl Error for ProcessError {
    fn cause(&self) -> Option<&dyn Error> {
        match self {
            ProcessError::Unexpected(_, _)
            | ProcessError::Duplicate(_, _)
            | ProcessError::NoProcessor(_) => None,
            ProcessError::Trace(err, _) => Some(err),
            ProcessError::Collection(_) => None,
            ProcessError::Io(_, err) => Some(err),
        }
    }
}
