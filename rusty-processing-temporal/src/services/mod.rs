use async_once::AsyncOnce;
use aws_sdk_s3 as s3;
use lazy_static::lazy_static;

pub use archive_builder::*;

mod archive_builder;

lazy_static! {
    static ref S3_CLIENT: AsyncOnce<s3::Client> = AsyncOnce::new(async {
        let config = aws_config::load_from_env().await;
        s3::Client::new(&config)
    });
}

pub async fn s3_client() -> &'static s3::Client {
    S3_CLIENT.get().await
}
