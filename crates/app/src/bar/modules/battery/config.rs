//! Battery module configuration.

use serde::{Deserialize, Serialize};

/// Battery module configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BatteryConfig {
    pub show_icon: bool,
    pub show_percentage: bool,
}

impl Default for BatteryConfig {
    fn default() -> Self {
        Self {
            show_icon: true,
            show_percentage: true,
        }
    }
}
