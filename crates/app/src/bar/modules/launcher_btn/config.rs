//! Launcher button module configuration.

use serde::{Deserialize, Serialize};

/// Launcher button module configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherBtnConfig {
    pub icon: String,
}

impl Default for LauncherBtnConfig {
    fn default() -> Self {
        Self {
            icon: "󰀻".into()
        }
    }
}
