use mail_parser::ContentType;
pub use processor::*;

mod html_message_visitor;
mod message_formatter;
mod message_visitor;
mod processor;
mod text_message_visitor;
mod transformer;

mod text;
mod metadata;
mod pdf;

/// Get the MIME type from a `mail_parser::ContentType`.
///
/// # Arguments
///
/// * `content_type` - The `mail_parser::ContentType` to get the MIME type from.
///
/// # Returns
///
/// The MIME type formatted as a `String`.
///
pub fn mimetype(content_type: &ContentType) -> String {
    match (content_type.ctype(), content_type.subtype()) {
        (ctype, Some(subtype)) => format!("{}/{}", ctype, subtype),
        (ctype, None) => ctype.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use mail_parser::ContentType;
    use super::*;

    #[test]
    fn test_mimetype_with_subtype() {
        let content_type = ContentType {
            c_type: Cow::from("text"),
            c_subtype: Some(Cow::from("plain")),
            attributes: None,
        };

        assert_eq!(mimetype(&content_type), "text/plain");
    }

    #[test]
    fn test_mimetype_without_subtype() {
        let content_type = ContentType {
            c_type: Cow::from("text"),
            c_subtype: None,
            attributes: None,
        };

        assert_eq!(mimetype(&content_type), "text");
    }
}