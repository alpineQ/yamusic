use std::path::PathBuf;
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
    #[serde(rename = "cache-dir")]
    pub cache_dir: Option<String>,
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

pub fn cache_dir() -> PathBuf {
    if let Some(dir) = get().cache_dir.as_deref() {
        return expand_tilde(dir);
    }
    ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))
        .map(|d| d.cache_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(path)
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
        assert_eq!(cfg.cache_dir, None);
    }

    #[test]
    fn parses_cache_dir() {
        let cfg: Config = toml::from_str("cache-dir = \"/mnt/data4/.yamusic\"").unwrap();
        assert_eq!(cfg.cache_dir.as_deref(), Some("/mnt/data4/.yamusic"));
    }
}
