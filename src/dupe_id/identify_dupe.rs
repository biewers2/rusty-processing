use std::borrow::Cow;
use std::sync::Mutex;

pub type IdentifyDupeService = Mutex<Box<dyn IdentifyDupe>>;

pub trait IdentifyDupe: Send {
  fn identify(&self, raw: &[u8]) -> String;
}