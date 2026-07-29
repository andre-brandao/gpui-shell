//! Launcher button module configuration.

use serde::{Deserialize, Serialize};
use ui::IconName;

/// Launcher button module configuration.
///
/// `icon` names an entry in the embedded icon set, e.g. `icon = "layers"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherBtnConfig {
    pub icon: IconName,
}

impl Default for LauncherBtnConfig {
    fn default() -> Self {
        Self {
            icon: super::LAUNCHER_ICON,
        }
    }
}
