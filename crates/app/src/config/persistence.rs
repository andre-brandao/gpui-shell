use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::Config;

/// Directory holding every file the shell reads or writes.
pub(super) fn config_dir() -> anyhow::Result<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(xdg).join("gpuishell"));
    }

    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home).join(".config").join("gpuishell"));
    }

    Err(anyhow!(
        "Unable to determine config path (XDG_CONFIG_HOME/HOME not set)"
    ))
}

pub fn config_path() -> anyhow::Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

pub fn load() -> anyhow::Result<Config> {
    let path = config_path()?;
    match parse_toml(&path)? {
        Some(config) => Ok(config),
        None => {
            let config = Config::default();
            write_toml(&path, &config)?;
            Ok(config)
        }
    }
}

/// Read and parse a TOML file. `Ok(None)` when it isn't there, so every caller
/// decides for itself what an absent file means.
pub(super) fn parse_toml<T: DeserializeOwned>(path: &Path) -> anyhow::Result<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }

    let raw =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let parsed =
        toml::from_str(&raw).with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(Some(parsed))
}

/// Serialize `value` into `path`, creating the config directory if needed.
pub(super) fn write_toml<T: Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("Invalid path has no parent directory: {}", path.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create directory: {}", parent.display()))?;

    let encoded = toml::to_string_pretty(value).context("Failed to encode TOML")?;
    fs::write(path, encoded)
        .with_context(|| format!("Failed to write file: {}", path.display()))?;
    Ok(())
}
