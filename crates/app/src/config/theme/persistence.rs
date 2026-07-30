use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use ui::{StoredTheme, Theme};

use crate::config::persistence::{config_dir, write_toml};

pub fn theme_path() -> anyhow::Result<PathBuf> {
    Ok(config_dir()?.join("theme.toml"))
}

/// Read `theme.toml`. A theme we can't parse is reported, not repaired: it
/// used to be overwritten with the defaults here, which meant saving a
/// half-edited file - the hot reload fires on every save - destroyed it.
pub fn load_theme() -> anyhow::Result<Theme> {
    let path = theme_path()?;
    if !path.exists() {
        return Ok(Theme::default());
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read theme file: {}", path.display()))?;
    let parsed = toml::from_str::<StoredTheme>(&raw)
        .with_context(|| format!("Failed to parse theme file: {}", path.display()))?;
    Ok(parsed.into_theme())
}

pub fn save_theme(theme: &Theme) -> anyhow::Result<()> {
    write_toml(&theme_path()?, &StoredTheme::from_theme(theme))
}
