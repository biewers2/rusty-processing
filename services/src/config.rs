use lazy_static::lazy_static;
lazy_static! {
    static ref CONFIG: Config = Config;
}

/// A singleton for accessing global configuration values.
///
pub fn config() -> &'static Config {
    &CONFIG
}

/// A struct used to define an interface for accessing embedded-wide configuration values.
///
#[derive(Debug, Clone, Default)]
pub struct Config;

impl Config {
    /// Get the value of an environment variable.
    ///
    /// # Arguments
    ///
    /// * `key` - The name of the environment variable.
    ///
    /// # Returns
    ///
    /// The value of the environment variable, or [`None`] if it is not set.
    ///
    pub fn get(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }

    /// Get the value of an environment variable, or a default value.
    ///
    /// # Arguments
    ///
    /// * `key` - The name of the environment variable.
    /// * `default` - The default value to return if the environment variable is not set.
    ///
    /// # Returns
    ///
    /// The value of the environment variable, or the default value if it is not set.
    ///
    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.get(key).unwrap_or_else(|| default.to_string())
    }
}