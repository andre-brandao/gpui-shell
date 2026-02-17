//! Settings module configuration.

use serde::{Deserialize, Serialize};

/// Settings module configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SettingsConfig {}

impl Default for SettingsConfig {
    fn default() -> Self {
        Self {}
    }
}
