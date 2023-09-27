/// Defines a type for a boxed [`Identify`] implementation.
///
pub type IdentifyDedupeService = Box<dyn Identify>;

/// Defines the interface for a duplicate file identification service.
///
pub trait Identify: Send + Sync {
    /// Identifies duplicate file by producing a unique identifier for the file.
    ///
    fn identify(&self, raw: &[u8]) -> String;
}
