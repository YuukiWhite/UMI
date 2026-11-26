pub mod schema;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

pub use schema::*;

pub fn config_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("org", "umi", "umi")
        .context("Failed to resolve XDG config directory")?;
    Ok(dirs.config_dir().join("config.toml"))
}

pub fn load_or_create() -> Result<UmiConfig> {
    let path = config_path()?;
    if !path.exists() {
        let cfg = UmiConfig::default();
        save(&cfg)?;
        return Ok(cfg);
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file {}", path.display()))?;
    let cfg: UmiConfig = toml::from_str(&raw)
        .with_context(|| format!("Failed to parse TOML config {}", path.display()))?;
    Ok(cfg)
}

pub fn save(cfg: &UmiConfig) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config dir {}", parent.display()))?;
    }
    let body = toml::to_string_pretty(cfg).context("Failed to serialize UmiConfig")?;
    fs::write(&path, body)
        .with_context(|| format!("Failed to write config file {}", path.display()))?;
    Ok(())
}
