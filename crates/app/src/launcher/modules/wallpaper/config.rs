//! Wallpaper view configuration.

use serde::{Deserialize, Serialize};

/// Wallpaper view configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WallpaperConfig {
    pub prefix: String,
    pub directory: String,
    pub matugen_enabled: bool,
    pub matugen_mode: String,
    pub matugen_type: String,
    pub matugen_source_color_index: usize,
}

impl Default for WallpaperConfig {
    fn default() -> Self {
        Self {
            prefix: ";wp".into(),
            directory: "~/Pictures/Wallpapers".into(),
            matugen_enabled: true,
            matugen_mode: "dark".into(),
            matugen_type: "scheme-tonal-spot".into(),
            matugen_source_color_index: 0,
        }
    }
}
