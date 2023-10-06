use std::io::Cursor;

struct IdentifyTestCase {
    path: String,
    expected_mimetype: String,
    expected_checksum: String,
}

impl IdentifyTestCase {
    fn new(path: impl AsRef<str>, expected_mimetype: impl AsRef<str>, expected_checksum: impl AsRef<str>) -> Self {
        Self {
            path: Self::resource(path.as_ref()),
            expected_checksum: expected_checksum.as_ref().to_string(),
            expected_mimetype: expected_mimetype.as_ref().to_string(),
        }
    }

    fn resource(path: &str) -> String {
        format!("../resources/{}", path)
    }
}

struct IdentifyAssertion {
    checksum: (String, String),
    mimetype: (String, String),
}

#[tokio::test]
async fn test_identify() {
    let test_cases = vec![
        IdentifyTestCase::new("jpg/jQuery-text.jpg", "image/jpeg", "83eae4d02dd10d7f665e886fc538bd7d"),
        IdentifyTestCase::new("mbox/ubuntu-no-small.mbox", "application/mbox", "9803afc9f392a3efa87ac6103320a72c"),
        IdentifyTestCase::new("pdf/Espresso Machine Cleaning Guide.pdf", "application/pdf", "477abb4bafa9be1d3a40c8f22e59e9f7"),
        IdentifyTestCase::new("rfc822/headers-small.eml", "message/rfc822", "1da0ecb4e351b6dce0e41dafc0833430"),
        IdentifyTestCase::new("zip/testzip.zip", "application/zip", "4f0b4e98af1c1a43e96a73b26ee0829b"),
    ];

    let handles = test_cases.into_iter()
        .map(identify)
        .collect::<Vec<tokio::task::JoinHandle<IdentifyAssertion>>>();

    for handle in handles {
        let assertion = handle.await.unwrap();

        let (actual_checksum, expected_checksum) = assertion.checksum;
        assert_eq!(actual_checksum, expected_checksum);

        let (actual_mimetype, expected_mimetype) = assertion.mimetype;
        assert_eq!(actual_mimetype, expected_mimetype);
    }
}

fn identify(test_case: IdentifyTestCase) -> tokio::task::JoinHandle<IdentifyAssertion> {
    tokio::spawn(async move {
        let mut file = tokio::fs::File::open(test_case.path).await.unwrap();
        let dedupe = identify::deduplication::dedupe_checksum(&mut file).await.unwrap();
        IdentifyAssertion {
            checksum: (dedupe.checksum, test_case.expected_checksum),
            mimetype: (dedupe.mimetype, test_case.expected_mimetype),
        }
    })
}
