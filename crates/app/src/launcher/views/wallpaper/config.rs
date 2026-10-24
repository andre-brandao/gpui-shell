//! Wallpaper view configuration.

use serde::{Deserialize, Serialize};

/// Wallpaper view configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WallpaperConfig {
    pub prefix: String,
    pub directory: String,
    /// Cards per grid row.
    pub columns: usize,
    /// Generate dark and light themes from an applied wallpaper with
    /// matugen, offered above the grid. Never applied on its own.
    pub matugen: bool,
    /// matugen `--type`, e.g. `scheme-tonal-spot`, `scheme-content`.
    pub matugen_scheme: String,
}

impl Default for WallpaperConfig {
    fn default() -> Self {
        Self {
            prefix: ";wp".into(),
            directory: "~/Pictures/Wallpapers".into(),
            columns: 3,
            matugen: true,
            matugen_scheme: "scheme-tonal-spot".into(),
        }
    }
}
