use std::fs::File;
use std::io::Write;
use std::path;

use anyhow::anyhow;
use mail_parser::Message;

use crate::common::wkhtmltopdf::wkhtmltopdf;
use crate::message::rfc822::html_message_visitor::HtmlMessageVisitor;
use crate::message::rfc822::processor::Rfc822Processor;
use crate::message::rfc822::transformer::MessageTransformer;

impl Rfc822Processor {
    pub fn render_pdf(&self, message: &Message, output_path: &path::PathBuf) -> anyhow::Result<()> {
        let transformer = MessageTransformer::new(Box::<HtmlMessageVisitor>::default());

        let mut html = Vec::<u8>::new();
        let mut pdf: Vec<u8> = Vec::new();

        transformer.transform(message, &mut html)?;
        self.render_html_to_pdf(html.to_vec(), &mut pdf)?;
        self.write_pdf_to_file(&output_path, pdf)
    }

    fn render_html_to_pdf(&self, html: Vec<u8>, output: &mut Vec<u8>) -> anyhow::Result<()> {
        let status = wkhtmltopdf().run(html.as_ref(), output)?;
        if !status.success() && status.code().is_some_and(|code| code != 1) {
            Err(anyhow!("wkhtmltopdf exited with status {}", status))?;
        }
        Ok(())
    }

    fn write_pdf_to_file(&self, output_path: &path::PathBuf, pdf: Vec<u8>) -> anyhow::Result<()> {
        File::create(output_path).and_then(|mut file| file.write_all(pdf.as_ref()))?;
        Ok(())
    }
}
