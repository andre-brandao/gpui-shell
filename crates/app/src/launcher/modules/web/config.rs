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
            providers: vec![
                WebProviderConfig {
                    shebang: "ddg".into(),
                    name: "DuckDuckGo".into(),
                    icon: "\u{f535}".into(),
                    url: "https://duckduckgo.com/?q={query}".into(),
                    default: true,
                },
                WebProviderConfig {
                    shebang: "g".into(),
                    name: "Google".into(),
                    icon: "\u{f1a0}".into(),
                    url: "https://www.google.com/search?q={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "yt".into(),
                    name: "YouTube".into(),
                    icon: "\u{f167}".into(),
                    url: "https://www.youtube.com/results?search_query={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "gh".into(),
                    name: "GitHub".into(),
                    icon: "\u{f09b}".into(),
                    url: "https://github.com/search?q={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "nix".into(),
                    name: "Nixpkgs".into(),
                    icon: "\u{f313}".into(),
                    url: "https://search.nixos.org/packages?query={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "w".into(),
                    name: "Wikipedia".into(),
                    icon: "\u{f266}".into(),
                    url: "https://en.wikipedia.org/wiki/Special:Search?search={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "rs".into(),
                    name: "crates.io".into(),
                    icon: "\u{e7a8}".into(),
                    url: "https://crates.io/search?q={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "r".into(),
                    name: "Reddit".into(),
                    icon: "\u{f281}".into(),
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
    #[serde(default)]
    pub icon: String,
    pub url: String,
    #[serde(default)]
    pub default: bool,
}
