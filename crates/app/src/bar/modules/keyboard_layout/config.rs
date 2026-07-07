//! Keyboard layout module configuration.

use serde::{Deserialize, Serialize};

/// Keyboard layout module configuration.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardLayoutConfig {
    pub show_flag: bool,
}
