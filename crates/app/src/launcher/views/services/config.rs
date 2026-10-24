//! Configuration for services status view.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServicesConfig {
    pub prefix: String,
}

impl Default for ServicesConfig {
    fn default() -> Self {
        Self {
            prefix: ";s".to_string(),
        }
    }
}
