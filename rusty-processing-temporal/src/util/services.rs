use async_once::AsyncOnce;
use lazy_static::lazy_static;
use aws_sdk_s3 as s3;

lazy_static! {
    static ref S3_CLIENT: AsyncOnce<s3::Client> = AsyncOnce::new(async {
        let config = aws_config::load_from_env().await;
        let _client = s3::Client::new(&config);
        s3::Client::new(&config)
    });
}

pub async fn s3_client() -> &'static s3::Client {
    S3_CLIENT.get().await
}
