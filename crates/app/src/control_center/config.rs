//! Control Center configuration.

use serde::{Deserialize, Serialize};

/// Power action commands for the Control Center.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PowerActionsConfig {
    pub sleep: String,
    pub reboot: String,
    pub poweroff: String,
}

impl Default for PowerActionsConfig {
    fn default() -> Self {
        Self {
            sleep: "systemctl suspend".to_string(),
            reboot: "systemctl reboot".to_string(),
            poweroff: "systemctl poweroff".to_string(),
        }
    }
}

/// Control Center configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlCenterConfig {
    pub power_actions: PowerActionsConfig,
}

impl Default for ControlCenterConfig {
    fn default() -> Self {
        Self {
            power_actions: PowerActionsConfig::default(),
        }
    }
}
