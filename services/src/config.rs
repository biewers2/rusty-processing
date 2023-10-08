use lazy_static::lazy_static;
lazy_static! {
    static ref CONFIG: Config = Config;
}

pub fn config() -> &'static Config {
    &CONFIG
}

#[derive(Debug, Clone, Default)]
pub struct Config;

impl Config {
    pub fn get(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }

    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.get(key).unwrap_or_else(|| default.to_string())
    }
}