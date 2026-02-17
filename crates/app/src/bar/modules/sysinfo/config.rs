//! SysInfo module configuration.

use serde::{Deserialize, Serialize};

/// SysInfo module configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SysInfoConfig {
    pub show_cpu: bool,
    pub show_memory: bool,
    pub show_temp: bool,
}

impl Default for SysInfoConfig {
    fn default() -> Self {
        Self {
            show_cpu: true,
            show_memory: true,
            show_temp: false,
        }
    }
}
