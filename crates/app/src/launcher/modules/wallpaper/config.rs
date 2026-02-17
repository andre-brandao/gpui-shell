//! Wallpaper view configuration.

use serde::{Deserialize, Serialize};

/// Wallpaper view configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WallpaperConfig {
    pub prefix: String,
    pub directory: String,
}

impl Default for WallpaperConfig {
    fn default() -> Self {
        Self {
            prefix: ";wp".into(),
            directory: "~/Pictures/Wallpapers".into(),
        }
    }
}
