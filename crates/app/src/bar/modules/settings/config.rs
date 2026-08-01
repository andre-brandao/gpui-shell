//! Settings module configuration.

use serde::{Deserialize, Serialize};

/// Settings module configuration.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SettingsConfig {}
