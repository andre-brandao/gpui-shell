//! Web search view configuration.

use serde::{Deserialize, Serialize};
use ui::{IconName, IconSource};

use crate::icons::{self, ConfigIcon};

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
            providers: vec![
                WebProviderConfig {
                    shebang: "ddg".into(),
                    name: "DuckDuckGo".into(),
                    icon: Some(ConfigIcon::Embedded(IconName::Duckduckgo)),
                    url: "https://duckduckgo.com/?q={query}".into(),
                    default: true,
                },
                WebProviderConfig {
                    shebang: "g".into(),
                    name: "Google".into(),
                    icon: Some(ConfigIcon::Embedded(IconName::Google)),
                    url: "https://www.google.com/search?q={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "yt".into(),
                    name: "YouTube".into(),
                    icon: Some(ConfigIcon::Embedded(IconName::Youtube)),
                    url: "https://www.youtube.com/results?search_query={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "gh".into(),
                    name: "GitHub".into(),
                    icon: Some(ConfigIcon::Embedded(IconName::Github)),
                    url: "https://github.com/search?q={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "nix".into(),
                    name: "Nixpkgs".into(),
                    icon: Some(ConfigIcon::Embedded(IconName::Nixos)),
                    url: "https://search.nixos.org/packages?query={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "w".into(),
                    name: "Wikipedia".into(),
                    icon: Some(ConfigIcon::Embedded(IconName::Wikipedia)),
                    url: "https://en.wikipedia.org/wiki/Special:Search?search={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "rs".into(),
                    name: "crates.io".into(),
                    // crates.io has no mark of its own, so Rust's stands in.
                    icon: Some(ConfigIcon::Embedded(IconName::Rust)),
                    url: "https://crates.io/search?q={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "r".into(),
                    name: "Reddit".into(),
                    icon: Some(ConfigIcon::Embedded(IconName::Reddit)),
                    url: "https://www.reddit.com/search?q={query}".into(),
                    default: false,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebProviderConfig {
    pub shebang: String,
    pub name: String,
    /// A name from the embedded icon set, or a path to your own file.
    /// Omit it - or give something we can't resolve - to fall back to a
    /// generic globe.
    #[serde(
        default,
        deserialize_with = "crate::icons::deserialize_lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub icon: Option<ConfigIcon>,
    pub url: String,
    #[serde(default)]
    pub default: bool,
}

impl WebProviderConfig {
    pub fn icon(&self) -> IconSource {
        icons::source_or(self.icon.as_ref(), IconName::Globe)
    }
}
