//! Settings module configuration.

use serde::{Deserialize, Serialize};

/// Settings module configuration.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SettingsConfig {}
