use std::sync::OnceLock;

use directories::ProjectDirs;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheMode {
    Off,
    #[default]
    Hotkey,
    Auto,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub cache: CacheMode,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn init() {
    let _ = CONFIG.set(load().unwrap_or_default());
}

pub fn get() -> &'static Config {
    CONFIG.get_or_init(Config::default)
}

pub fn cache_mode() -> CacheMode {
    get().cache
}

fn load() -> Option<Config> {
    let path = ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))?
        .config_dir()
        .join("config.toml");
    let text = std::fs::read_to_string(path).ok()?;
    match toml::from_str(&text) {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            tracing::warn!("config: parse failed, using defaults: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cache_mode() {
        let cfg: Config = toml::from_str("cache = \"auto\"").unwrap();
        assert_eq!(cfg.cache, CacheMode::Auto);
        let cfg: Config = toml::from_str("cache = \"off\"").unwrap();
        assert_eq!(cfg.cache, CacheMode::Off);
    }

    #[test]
    fn defaults_to_hotkey() {
        let cfg: Config = toml::from_str("").unwrap();
        assert_eq!(cfg.cache, CacheMode::Hotkey);
    }
}
