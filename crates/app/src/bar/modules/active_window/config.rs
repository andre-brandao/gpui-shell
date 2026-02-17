//! Active window module configuration.

use serde::{Deserialize, Serialize};

/// Active window module configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ActiveWindowConfig {
    pub max_length: usize,
    pub show_app_icon: bool,
}

impl Default for ActiveWindowConfig {
    fn default() -> Self {
        Self {
            max_length: 64,
            show_app_icon: true,
        }
    }
}
