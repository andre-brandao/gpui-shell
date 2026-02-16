use std::fs;
use std::path::PathBuf;

use anyhow::{Context, anyhow};

use super::Config;

fn default_config_path() -> anyhow::Result<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(xdg).join("gpuishell").join("config.toml"));
    }

    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home)
            .join(".config")
            .join("gpuishell")
            .join("config.toml"));
    }

    Err(anyhow!(
        "Unable to determine config path (XDG_CONFIG_HOME/HOME not set)"
    ))
}

pub fn config_path() -> anyhow::Result<PathBuf> {
    default_config_path()
}

pub fn load() -> anyhow::Result<Config> {
    let path = default_config_path()?;
    if !path.exists() {
        let config = Config::default();
        save(&config)?;
        return Ok(config);
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let parsed = toml::from_str::<Config>(&raw)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
    Ok(parsed)
}

pub fn save(config: &Config) -> anyhow::Result<()> {
    let path = default_config_path()?;
    let parent = path.parent().ok_or_else(|| {
        anyhow!(
            "Invalid config path has no parent directory: {}",
            path.display()
        )
    })?;
    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;

    let encoded = toml::to_string_pretty(config).context("Failed to encode config as TOML")?;
    fs::write(&path, encoded)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;
    Ok(())
}
