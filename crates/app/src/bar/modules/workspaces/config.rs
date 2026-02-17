//! Workspaces module configuration.

use serde::{Deserialize, Serialize};

/// Workspaces module configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspacesConfig {
    pub show_icons: bool,
    pub show_numbers: bool,
}

impl Default for WorkspacesConfig {
    fn default() -> Self {
        Self {
            show_icons: true,
            show_numbers: true,
        }
    }
}
