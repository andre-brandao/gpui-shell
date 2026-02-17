//! Tray module configuration.

use serde::{Deserialize, Serialize};

/// Tray module configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TrayConfig {
    pub icon_size: f32,
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self { icon_size: 16.0 }
    }
}
