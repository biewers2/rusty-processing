use services::tika;

use streaming::stream_to_string;

#[tokio::test]
async fn test_tika_server_connection() {
    assert!(tika().is_connected().await);
}

#[tokio::test]
async fn test_tika_text() -> anyhow::Result<()> {
    let expected_text = "
Daily

Clean case panels, frame, and drip tray

Empty portafilter after use and rinse
with hot water before reinserting into
group

Weekly

While hot, scrub grouphead w/ brush

Backflush w/ water

Soak portafilter and basket in hot water
or cleaner

Monthly

Take off grouphead gasket and diffuser,
inspect, and clean

Backflush w/ cleaner


";

    let input_path = "../resources/pdf/Espresso Machine Cleaning Guide.pdf";
    let file = tokio::fs::File::open(input_path).await?;

    let (stream, streaming) = tika().text(file).await?;
    let streaming = tokio::spawn(streaming);

    let text = stream_to_string(stream).await;
    streaming.await??;

    assert_eq!(text, expected_text);
    Ok(())
}

#[tokio::test]
async fn test_tika_text_with_ocr() -> anyhow::Result<()> {
    let input_path = "../resources/jpg/jQuery-text.jpg";
    let file = tokio::fs::File::open(input_path).await?;

    let (stream, streaming) = tika().text(file).await?;
    let streaming = tokio::spawn(streaming);

    let text = stream_to_string(stream).await;
    streaming.await??;

    assert_eq!(text, "jQuery $%&U6~\n\n\n");
    Ok(())
}

#[tokio::test]
async fn test_tika_metadata() -> anyhow::Result<()> {
    let expected_metadata = "\
{\
\"X-TIKA:Parsed-By\":[\"org.apache.tika.parser.DefaultParser\",\"org.apache.tika.parser.mbox.MboxParser\"],\
\"X-TIKA:Parsed-By-Full-Set\":[\"org.apache.tika.parser.DefaultParser\",\"org.apache.tika.parser.mbox.MboxParser\"],\
\"Content-Encoding\":\"windows-1252\",\
\"language\":\"\",\
\"Content-Type\":\"application/mbox\"\
}";

    let input_path = "../resources/mbox/ubuntu-no-small.mbox";
    let file = tokio::fs::File::open(input_path).await?;

    let metadata = tika().metadata(file).await?;

    assert_eq!(metadata, expected_metadata);
    Ok(())
}

#[tokio::test]
async fn test_tika_detect() {
    let input_path = "../resources/zip/testzip.zip";
    let file = tokio::fs::File::open(input_path).await.unwrap();

    let mimetype = tika().detect(file).await.unwrap();
    assert_eq!(mimetype, "application/zip");
}
