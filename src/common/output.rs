use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
pub enum OutputType {
  Text,
  Metadata,
  Pdf
}

impl FromStr for OutputType {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "text" => Ok(OutputType::Text),
      "metadata" => Ok(OutputType::Metadata),
      "pdf" => Ok(OutputType::Pdf),
      _ => Err(format!("Can not convert {} to OutputType", s)),
    }
  }
}
