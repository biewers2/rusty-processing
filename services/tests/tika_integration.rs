use services::tika;

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
    let path = "../resources/pdf/Espresso Machine Cleaning Guide.pdf";

    let text = tika().text(path).await?;

    assert_eq!(text, expected_text);
    Ok(())
}

#[tokio::test]
async fn test_tika_text_with_ocr() -> anyhow::Result<()> {
    let path = "../resources/jpg/jQuery-text.jpg";

    let text = tika().text(path).await?;

    assert_eq!(text, "jQuery $%&U6~\n\n\n");
    Ok(())
}

#[tokio::test]
async fn test_tika_metadata() {
    let expected_metadata = "\
{\
\"X-TIKA:Parsed-By\":[\"org.apache.tika.parser.DefaultParser\",\"org.apache.tika.parser.mbox.MboxParser\"],\
\"X-TIKA:Parsed-By-Full-Set\":[\"org.apache.tika.parser.DefaultParser\",\"org.apache.tika.parser.mbox.MboxParser\"],\
\"Content-Encoding\":\"windows-1252\",\
\"language\":\"\",\
\"Content-Type\":\"application/mbox\"\
}";
    let path = "../resources/mbox/ubuntu-no-small.mbox";

    let metadata = tika().metadata(path).await.unwrap();

    assert_eq!(metadata, expected_metadata);
}

#[tokio::test]
async fn test_tika_detect() {
    let path = "../resources/zip/testzip.zip";

    let mimetype = tika().detect(path).await.unwrap();

    assert_eq!(mimetype, "application/zip");
}
