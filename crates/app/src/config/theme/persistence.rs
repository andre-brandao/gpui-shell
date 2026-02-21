use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context};
use ui::Theme;

use super::config::StoredTheme;

pub fn theme_path() -> anyhow::Result<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(xdg).join("gpuishell").join("theme.toml"));
    }

    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home)
            .join(".config")
            .join("gpuishell")
            .join("theme.toml"));
    }

    Err(anyhow!(
        "Unable to determine theme path (XDG_CONFIG_HOME/HOME not set)"
    ))
}

pub fn load_theme() -> anyhow::Result<Theme> {
    match try_load_theme() {
        Ok(theme) => Ok(theme),
        Err(e) => {
            tracing::warn!("Failed to load theme config: {e}, writing default");
            let default_theme = Theme::default();
            if let Err(write_err) = save_theme(&default_theme) {
                tracing::error!("Failed to write default theme: {write_err}");
            }
            Ok(default_theme)
        }
    }
}

fn try_load_theme() -> anyhow::Result<Theme> {
    let path = theme_path()?;
    if !path.exists() {
        return Ok(Theme::default());
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read theme file: {}", path.display()))?;
    let parsed = toml::from_str::<StoredTheme>(&raw)
        .with_context(|| format!("Failed to parse theme file: {}", path.display()))?;
    parsed.to_theme()
}

pub fn save_theme(theme: &Theme) -> anyhow::Result<()> {
    let path = theme_path()?;
    let parent = path.parent().ok_or_else(|| {
        anyhow!(
            "Invalid theme path has no parent directory: {}",
            path.display()
        )
    })?;
    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create theme directory: {}", parent.display()))?;

    let encoded = toml::to_string_pretty(&StoredTheme::from_theme(theme))
        .context("Failed to encode theme")?;
    fs::write(&path, encoded)
        .with_context(|| format!("Failed to write theme file: {}", path.display()))?;
    Ok(())
}
