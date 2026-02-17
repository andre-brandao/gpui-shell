//! Clock module configuration.

use serde::{Deserialize, Serialize};

/// Clock module configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClockConfig {
    pub format_horizontal: String,
    pub format_vertical: String,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            format_horizontal: "%d/%m/%Y %H:%M:%S".into(),
            format_vertical: "%H\n%M\n%S".into(),
        }
    }
}
