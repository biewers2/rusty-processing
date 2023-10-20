use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::anyhow;
use futures::future::try_join_all;
use redis::{AsyncCommands, RedisError, RedisResult};
use url::{ParseError, Url};

use crate::redis;

pub fn parse_s3_uri(s3_uri_str: impl AsRef<Path>) -> anyhow::Result<(String, String)> {
    let s3_uri_str = s3_uri_str.as_ref().to_string_lossy().to_string();
    let source_url = Url::from_str(s3_uri_str.as_str())
        .map_err(|_| anyhow!("Failed to parse S3 URL"))?;

    if let (Some(bucket), key) = (source_url.host(), source_url.path()) {
        let key = if let Some(stripped) = key.strip_prefix('/') {
            stripped
        } else {
            key
        };

        Ok((bucket.to_string(), key.to_string()))
    } else {
        Err(ParseError::EmptyHost)?
    }
}

pub struct BatchEntry {
    pub path: PathBuf,
    pub mimetype: String,
    pub checksum: String,
}

pub struct ProcessOutputBatcher<'a> {
    redis: redis::Client,
    stream: &'a str,
    batch_size: usize,
    batch: Vec<[(&'static str, String); 3]>,
}

impl<'a> ProcessOutputBatcher<'a> {
    pub fn new(stream: &'a str, batch_size: usize) -> Self {
        Self {
            redis: redis().clone(),
            stream,
            batch_size,
            batch: Vec::with_capacity(batch_size),
        }
    }

    pub async fn push(&mut self, entry: BatchEntry) -> Result<(), RedisError> {
        self.batch.push([
            ("path", entry.path.to_string_lossy().to_string()),
            ("mimetype", entry.mimetype),
            ("checksum", entry.checksum),
        ]);
        if self.batch.len() >= self.batch_size {
            self.flush().await?;
        }
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), RedisError> {
        if !self.batch.is_empty() {
            let mut submissions = vec![];

            for entry in &self.batch {
                submissions.push( async {
                    let mut conn = self.redis.get_async_connection().await?;
                    conn.xadd(self.stream, "*", entry).await?;
                    RedisResult::Ok(())
                })
            }

            try_join_all(submissions).await?;
            self.batch.clear();
        }
        Ok(())
    }
}
