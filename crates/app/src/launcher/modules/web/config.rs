//! Web search view configuration.

use serde::{Deserialize, Serialize};
use ui::IconName;

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
                    icon: Some(IconName::MagnifyingGlass),
                    url: "https://duckduckgo.com/?q={query}".into(),
                    default: true,
                },
                WebProviderConfig {
                    shebang: "g".into(),
                    name: "Google".into(),
                    icon: Some(IconName::MagnifyingGlass),
                    url: "https://www.google.com/search?q={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "yt".into(),
                    name: "YouTube".into(),
                    icon: Some(IconName::Play),
                    url: "https://www.youtube.com/results?search_query={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "gh".into(),
                    name: "GitHub".into(),
                    icon: Some(IconName::GitBranch),
                    url: "https://github.com/search?q={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "nix".into(),
                    name: "Nixpkgs".into(),
                    icon: Some(IconName::Layers),
                    url: "https://search.nixos.org/packages?query={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "w".into(),
                    name: "Wikipedia".into(),
                    icon: Some(IconName::BookOpen),
                    url: "https://en.wikipedia.org/wiki/Special:Search?search={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "rs".into(),
                    name: "crates.io".into(),
                    icon: Some(IconName::Hexagon),
                    url: "https://crates.io/search?q={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "r".into(),
                    name: "Reddit".into(),
                    icon: Some(IconName::Chat),
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
    /// Named from the embedded icon set. Omit it - or give a name we don't
    /// ship - to fall back to a generic globe.
    #[serde(
        default,
        deserialize_with = "crate::icons::deserialize_lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub icon: Option<IconName>,
    pub url: String,
    #[serde(default)]
    pub default: bool,
}

impl WebProviderConfig {
    pub fn icon(&self) -> IconName {
        self.icon.unwrap_or(IconName::Globe)
    }
}
