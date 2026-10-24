//! Apps view configuration.

use serde::{Deserialize, Serialize};

/// Apps view configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppsConfig {
    pub prefix: String,
}

impl Default for AppsConfig {
    fn default() -> Self {
        Self { prefix: "@".into() }
    }
}
