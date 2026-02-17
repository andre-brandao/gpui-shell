//! Web search view configuration.

use serde::{Deserialize, Serialize};

/// Web search view configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebConfig {
    pub prefix: String,
    pub providers: Vec<WebProviderConfig>,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            prefix: "!".into(),
            providers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebProviderConfig {
    pub shebang: String,
    pub name: String,
    #[serde(default)]
    pub icon: String,
    pub url: String,
    #[serde(default)]
    pub default: bool,
}
