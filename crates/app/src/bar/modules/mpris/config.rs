//! MPRIS module configuration.

use serde::{Deserialize, Serialize};

/// MPRIS module configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MprisConfig {
    pub show_cover: bool,
    pub max_width: f32,
}

impl Default for MprisConfig {
    fn default() -> Self {
        Self {
            show_cover: true,
            max_width: 220.0,
        }
    }
}
