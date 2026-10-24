//! Keyboard layout module configuration.

use serde::{Deserialize, Serialize};

/// Keyboard layout module configuration.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardLayoutConfig {
    pub show_flag: bool,
}
