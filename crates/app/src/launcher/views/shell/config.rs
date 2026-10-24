//! Shell view configuration.

use serde::{Deserialize, Serialize};

/// Shell view configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellConfig {
    pub prefix: String,
    pub terminal: String,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            prefix: "$".into(),
            terminal: String::new(),
        }
    }
}
