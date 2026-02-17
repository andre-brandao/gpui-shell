//! Keyboard layout module configuration.

use serde::{Deserialize, Serialize};

/// Keyboard layout module configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardLayoutConfig {
    pub show_flag: bool,
}

impl Default for KeyboardLayoutConfig {
    fn default() -> Self {
        Self { show_flag: false }
    }
}
