/// Defines a type for a boxed [`IdentifyDupe`] implementation.
///
pub type IdentifyDupeService = Box<dyn IdentifyDupe>;

/// Defines the interface for a duplicate file identification service.
///
pub trait IdentifyDupe: Send + Sync {
    /// Identifies duplicate file by producing a unique identifier for the file.
    ///
    fn identify(&self, raw: &[u8]) -> String;
}
