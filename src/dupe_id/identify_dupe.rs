pub type IdentifyDupeService = Box<dyn IdentifyDupe>;

pub trait IdentifyDupe: Send + Sync {
  fn identify(&self, raw: &[u8]) -> String;
}