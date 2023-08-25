use std::path;
use std::fs::File;
use std::io::Write;

use mail_parser::Message;

use crate::common::error::{ProcessError, ProcessResult};
use crate::common::wkhtmltopdf::wkhtmltopdf;
use crate::message::rfc822::html_message_visitor::HtmlMessageVisitor;
use crate::message::rfc822::transformer::MessageTransformer;

pub fn render(message: &Message, output_path: path::PathBuf) -> ProcessResult<()> {
  let transformer = MessageTransformer::new(Box::<HtmlMessageVisitor>::default());

  let mut html = Vec::<u8>::new();
  let mut pdf: Vec<u8> = Vec::new();

  transformer.transform(message, &mut html)?;
  render_html_to_pdf(html.to_vec(), &mut pdf)?;
  write_pdf_to_file(&output_path, pdf)
}

fn render_html_to_pdf(html: Vec<u8>, output: &mut Vec<u8>) -> ProcessResult<()> {
  wkhtmltopdf()
    .run(html.as_ref(), output)
    .map_err(|err| ProcessError::from_io(err, "Failed to run wkhtmltopdf"))
    .and_then(|status| {
      if status.success() || status.code().is_some_and(|code| code == 1) {
        Ok(())
      } else {
        Err(ProcessError::Unexpected(format!("wkhtmltopdf exited with status {}", status)))
      }
    })
}

fn write_pdf_to_file(output_path: &path::PathBuf, pdf: Vec<u8>) -> ProcessResult<()> {
  File::create(output_path)
    .and_then(|mut file| file.write_all(pdf.as_ref()))
    .map_err(|err| ProcessError::from_io(err, "Failed to write PDF file"))
}