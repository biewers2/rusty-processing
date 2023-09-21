pub async fn download<S>(source_s3_uri: impl AsRef<str>) -> anyhow::Result<()> {
    // let (bucket, key) = parse_s3_uri(source_s3_uri.as_ref())?;
    // let object = s3_client()
    //     .await
    //     .get_object()
    //     .bucket(bucket)
    //     .key(key)
    //     .send();
    //
    // Ok(object.await?.body)
    Ok(())
}
