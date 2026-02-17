//! Theme view configuration.

use serde::{Deserialize, Serialize};

/// Themes view configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemesConfig {
    pub prefix: String,
}

impl Default for ThemesConfig {
    fn default() -> Self {
        Self { prefix: "~".into() }
    }
}
