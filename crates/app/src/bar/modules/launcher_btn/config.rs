//! Launcher button module configuration.

use serde::{Deserialize, Serialize};
use ui::IconName;

/// Launcher button module configuration.
///
/// `icon` names an entry in the embedded icon set, e.g. `icon = "layers"`.
/// Omit it - or give a name we don't ship - to get the built-in icon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherBtnConfig {
    #[serde(
        deserialize_with = "crate::icons::deserialize_lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub icon: Option<IconName>,
}

impl Default for LauncherBtnConfig {
    fn default() -> Self {
        Self {
            icon: Some(super::LAUNCHER_ICON),
        }
    }
}
