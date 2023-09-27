use std::future::Future;

use bytes::Bytes;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_stream::wrappers::ReceiverStream;

pub use parse_s3_uri::*;
use rusty_processing::common::ByteStream;

mod parse_s3_uri;
