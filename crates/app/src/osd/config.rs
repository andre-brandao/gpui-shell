//! OSD module configuration.

use serde::{Deserialize, Serialize};

/// OSD screen position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OsdPosition {
    Top,
    Bottom,
    Left,
    #[default]
    Right,
}

/// OSD configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OsdConfig {
    pub position: OsdPosition,
}

impl Default for OsdConfig {
    fn default() -> Self {
        Self {
            position: OsdPosition::Right,
        }
    }
}
