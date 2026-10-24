//! Help view configuration.

use serde::{Deserialize, Serialize};

/// Help view configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HelpConfig {
    pub prefix: String,
}

impl Default for HelpConfig {
    fn default() -> Self {
        Self { prefix: "?".into() }
    }
}
