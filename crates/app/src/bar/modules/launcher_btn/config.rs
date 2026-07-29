//! Launcher button module configuration.

use serde::{Deserialize, Serialize};

use crate::icons::ConfigIcon;

/// Launcher button module configuration.
///
/// `icon` is either a name from the embedded set (`icon = "layers"`) or a
/// path to your own file (`icon = "~/.config/gpuishell/icons/mine.svg"`).
/// Omit it - or give something we can't resolve - to get the built-in icon.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherBtnConfig {
    #[serde(
        deserialize_with = "crate::icons::deserialize_lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub icon: Option<ConfigIcon>,
}
