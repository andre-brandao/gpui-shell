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
                    icon: None,
                    url: "https://duckduckgo.com/?q={query}".into(),
                    default: true,
                },
                WebProviderConfig {
                    shebang: "g".into(),
                    name: "Google".into(),
                    icon: None,
                    url: "https://www.google.com/search?q={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "yt".into(),
                    name: "YouTube".into(),
                    icon: None,
                    url: "https://www.youtube.com/results?search_query={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "gh".into(),
                    name: "GitHub".into(),
                    icon: None,
                    url: "https://github.com/search?q={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "nix".into(),
                    name: "Nixpkgs".into(),
                    icon: None,
                    url: "https://search.nixos.org/packages?query={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "w".into(),
                    name: "Wikipedia".into(),
                    icon: None,
                    url: "https://en.wikipedia.org/wiki/Special:Search?search={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "rs".into(),
                    name: "crates.io".into(),
                    icon: None,
                    url: "https://crates.io/search?q={query}".into(),
                    default: false,
                },
                WebProviderConfig {
                    shebang: "r".into(),
                    name: "Reddit".into(),
                    icon: None,
                    url: "https://www.reddit.com/search?q={query}".into(),
                    default: false,
                },
            ],
        }
    }
}

/// Brand mark for a provider we recognise, keyed on its URL.
///
/// This is the only thing standing between a hand-written `providers` list
/// and eight identical globes. A config that names its own providers
/// *replaces* the default list rather than merging into it, so marks written
/// only into [`WebConfig::default`] would serve a config that configures no
/// providers at all - and no other.
///
/// Keyed on the URL because that is what actually decides which service gets
/// searched; `name` and `shebang` are free text a user can set to anything.
const BRAND_MARKS: &[(&str, IconName)] = &[
    ("duckduckgo.", IconName::Duckduckgo),
    ("google.", IconName::Google),
    ("youtube.", IconName::Youtube),
    ("github.", IconName::Github),
    ("nixos.org", IconName::Nixos),
    ("wikipedia.", IconName::Wikipedia),
    ("crates.io", IconName::Rust),
    ("reddit.", IconName::Reddit),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebProviderConfig {
    pub shebang: String,
    pub name: String,
    /// A name from the embedded icon set, or a path to your own file. Omit
    /// it (or give something we can't resolve) and a provider we recognise
    /// by URL falls back to its brand mark, anything else to a generic globe.
    #[serde(
        default,
        deserialize_with = "crate::icons::deserialize_lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub icon: Option<ConfigIcon>,
    pub url: String,
    /// Which provider a bare `!` searches. Omitted means `false`; when no
    /// provider claims it, the first one in the list is used.
    #[serde(default)]
    pub default: bool,
}

impl WebProviderConfig {
    pub fn icon(&self) -> IconSource {
        icons::source_or(self.icon.as_ref(), self.brand_mark())
    }

    /// The mark for this provider's service, or a globe for one we don't know.
    fn brand_mark(&self) -> IconName {
        BRAND_MARKS
            .iter()
            .find(|(host, _)| self.url.contains(host))
            .map_or(IconName::Globe, |&(_, icon)| icon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A config that lists its own providers replaces the default list, so
    /// every entry reaches us with `icon: None`. Marks have to come from the
    /// provider itself, not from `WebConfig::default`.
    #[test]
    fn a_hand_written_provider_list_still_gets_brand_marks() {
        let hand_written: WebConfig = toml::from_str(
            r#"
            prefix = "!"
            [[providers]]
            shebang = "ddg"
            name = "DuckDuckGo"
            url = "https://duckduckgo.com/?q={query}"
            [[providers]]
            shebang = "rs"
            name = "crates.io"
            url = "https://crates.io/search?q={query}"
            [[providers]]
            shebang = "x"
            name = "Some Wiki"
            url = "https://example.com/?q={query}"
            "#,
        )
        .expect("parses");

        let marks: Vec<IconName> = hand_written
            .providers
            .iter()
            .map(|p| p.brand_mark())
            .collect();

        assert_eq!(
            marks,
            [
                IconName::Duckduckgo,
                IconName::Rust,
                // Not a service we know - a globe is the honest answer.
                IconName::Globe,
            ]
        );
    }

    /// `default` is optional, and every shipped provider has to resolve to its
    /// own mark - the default list carries no explicit icons any more, so a
    /// URL that stops matching `BRAND_MARKS` silently regresses to a globe.
    #[test]
    fn every_shipped_provider_resolves_to_a_brand_mark() {
        let shipped = WebConfig::default();
        let generic: Vec<&str> = shipped
            .providers
            .iter()
            .filter(|p| p.brand_mark() == IconName::Globe)
            .map(|p| p.name.as_str())
            .collect();

        assert!(
            generic.is_empty(),
            "provider(s) fell back to a globe: {generic:?}"
        );
    }

    #[test]
    fn an_omitted_default_flag_is_false() {
        let provider: WebProviderConfig = toml::from_str(
            r#"
            shebang = "x"
            name = "X"
            url = "https://example.com/?q={query}"
            "#,
        )
        .expect("parses without a default flag");

        assert!(!provider.default);
    }
}
