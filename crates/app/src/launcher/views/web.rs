//! Web search view for searching various web providers.
//!
//! Supports multiple search providers via "shebangs":
//! - `!g query` - Google
//! - `!yt query` - YouTube
//! - `!gh query` - GitHub
//! - `!nix query` - Nixpkgs
//! - `!ddg query` - DuckDuckGo
//! - `! query` - Default provider (DuckDuckGo)

use crate::launcher::view::{LauncherView, ViewContext};
use gpui::{AnyElement, App, FontWeight, div, prelude::*, px, rgba};
use ui::{bg, font_size, interactive, radius, spacing, text};

/// A search provider with its shebang and URL template.
struct SearchProvider {
    /// The shebang identifier (e.g., "g", "yt", "gh").
    shebang: &'static str,
    /// Display name.
    name: &'static str,
    /// Icon (Nerd font).
    icon: &'static str,
    /// URL template with `{query}` placeholder.
    url_template: &'static str,
    /// Whether this is the default provider.
    is_default: bool,
}

const PROVIDERS: &[SearchProvider] = &[
    SearchProvider {
        shebang: "g",
        name: "Google",
        icon: "󰊭",
        url_template: "https://www.google.com/search?q={query}",
        is_default: false,
    },
    SearchProvider {
        shebang: "yt",
        name: "YouTube",
        icon: "",
        url_template: "https://www.youtube.com/results?search_query={query}",
        is_default: false,
    },
    SearchProvider {
        shebang: "gh",
        name: "GitHub",
        icon: "",
        url_template: "https://github.com/search?q={query}",
        is_default: false,
    },
    SearchProvider {
        shebang: "nix",
        name: "Nixpkgs",
        icon: "",
        url_template: "https://search.nixos.org/packages?query={query}",
        is_default: false,
    },
    SearchProvider {
        shebang: "ddg",
        name: "DuckDuckGo",
        icon: "󰇥",
        url_template: "https://duckduckgo.com/?q={query}",
        is_default: true,
    },
    SearchProvider {
        shebang: "w",
        name: "Wikipedia",
        icon: "󰖬",
        url_template: "https://en.wikipedia.org/wiki/Special:Search?search={query}",
        is_default: false,
    },
    SearchProvider {
        shebang: "rs",
        name: "crates.io",
        icon: "",
        url_template: "https://crates.io/search?q={query}",
        is_default: false,
    },
    SearchProvider {
        shebang: "r",
        name: "Reddit",
        icon: "",
        url_template: "https://www.reddit.com/search?q={query}",
        is_default: false,
    },
];

/// Web search view - search the web with various providers.
pub struct WebSearchView;

impl WebSearchView {
    /// Parse the query to extract provider and search terms.
    /// Returns (provider, search_query).
    fn parse_query<'a>(&self, query: &'a str) -> (&'static SearchProvider, &'a str) {
        let query = query.trim();

        // Check for shebang at the start
        for provider in PROVIDERS {
            let prefix = provider.shebang;
            if query.starts_with(prefix) {
                let rest = &query[prefix.len()..];
                // Check if followed by space or end of string
                if rest.is_empty() || rest.starts_with(' ') {
                    return (provider, rest.trim());
                }
            }
        }

        // No shebang found, use default provider
        let default = PROVIDERS
            .iter()
            .find(|p| p.is_default)
            .unwrap_or(&PROVIDERS[0]);
        (default, query)
    }

    /// Get the default provider.
    #[allow(dead_code)]
    fn default_provider(&self) -> &'static SearchProvider {
        PROVIDERS
            .iter()
            .find(|p| p.is_default)
            .unwrap_or(&PROVIDERS[0])
    }
}

impl LauncherView for WebSearchView {
    fn prefix(&self) -> &'static str {
        "!"
    }

    fn name(&self) -> &'static str {
        "Web Search"
    }

    fn icon(&self) -> &'static str {
        "󰖟"
    }

    fn description(&self) -> &'static str {
        "Search the web (!g, !yt, !gh, !nix, !ddg)"
    }

    fn render(&self, vx: &ViewContext, _cx: &App) -> (AnyElement, usize) {
        let (provider, search_query) = self.parse_query(vx.query);
        let has_query = !search_query.is_empty();

        let element = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(spacing::MD))
            .p(px(spacing::MD))
            // Search preview
            .child(
                div()
                    .w_full()
                    .p(px(spacing::MD))
                    .bg(bg::secondary())
                    .rounded(px(radius::MD))
                    .flex()
                    .flex_col()
                    .gap(px(spacing::SM))
                    // Provider header with search action
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(spacing::SM))
                                    .child(
                                        div()
                                            .text_size(px(font_size::XL))
                                            .text_color(text::primary())
                                            .child(provider.icon),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(font_size::BASE))
                                            .text_color(text::primary())
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(provider.name),
                                    )
                                    .child(
                                        div()
                                            .px(px(6.))
                                            .py(px(2.))
                                            .rounded(px(4.))
                                            .bg(rgba(0x555555ff))
                                            .text_size(px(font_size::XS))
                                            .child(format!("!{}", provider.shebang)),
                                    ),
                            )
                            // Search hint (always visible, changes appearance when query exists)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(spacing::SM))
                                    .px(px(spacing::SM))
                                    .py(px(4.))
                                    .rounded(px(radius::SM))
                                    .when(has_query && vx.selected_index == 0, |el| {
                                        el.bg(rgba(0x3b82f6ff))
                                    })
                                    .when(has_query && vx.selected_index != 0, |el| {
                                        el.bg(interactive::default())
                                    })
                                    .when(!has_query, |el| el.bg(rgba(0x00000033)))
                                    .child(
                                        div()
                                            .text_size(px(font_size::SM))
                                            .text_color(if has_query {
                                                text::primary()
                                            } else {
                                                text::disabled()
                                            })
                                            .child("Search"),
                                    )
                                    .child(
                                        div()
                                            .px(px(4.))
                                            .py(px(2.))
                                            .rounded(px(3.))
                                            .bg(rgba(0x00000044))
                                            .text_size(px(font_size::XS))
                                            .text_color(if has_query {
                                                text::muted()
                                            } else {
                                                text::disabled()
                                            })
                                            .child("Enter"),
                                    ),
                            ),
                    )
                    // Search query display
                    .child(
                        div()
                            .w_full()
                            .p(px(spacing::SM))
                            .bg(rgba(0x00000066))
                            .rounded(px(radius::SM))
                            .text_size(px(font_size::BASE))
                            .text_color(if has_query {
                                text::primary()
                            } else {
                                text::placeholder()
                            })
                            .child(if has_query {
                                format!("\"{}\"", search_query)
                            } else {
                                "Type your search query...".to_string()
                            }),
                    ),
            )
            // Provider list (when no query or showing alternatives)
            .child(
                div()
                    .w_full()
                    .pt(px(spacing::SM))
                    .flex()
                    .flex_col()
                    .gap(px(spacing::XS))
                    .child(
                        div()
                            .text_size(px(font_size::XS))
                            .text_color(text::disabled())
                            .font_weight(FontWeight::MEDIUM)
                            .child("AVAILABLE PROVIDERS"),
                    )
                    .child(div().flex().flex_wrap().gap(px(spacing::SM)).children(
                        PROVIDERS.iter().map(|p| {
                            let is_active = p.shebang == provider.shebang;
                            div()
                                .px(px(spacing::SM))
                                .py(px(4.))
                                .rounded(px(radius::SM))
                                .when(is_active, |el| el.bg(rgba(0x3b82f6ff)))
                                .when(!is_active, |el| el.bg(interactive::default()))
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .child(div().text_size(px(font_size::SM)).child(p.icon))
                                .child(
                                    div()
                                        .text_size(px(font_size::SM))
                                        .child(format!("!{}", p.shebang)),
                                )
                        }),
                    )),
            )
            // Help text
            .child(
                div()
                    .w_full()
                    .pt(px(spacing::MD))
                    .flex()
                    .flex_col()
                    .gap(px(spacing::XS))
                    .child(
                        div()
                            .text_size(px(font_size::XS))
                            .text_color(text::disabled())
                            .font_weight(FontWeight::MEDIUM)
                            .child("USAGE"),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(text::muted())
                            .child("• Type !<shebang> <query> to search specific provider"),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(text::muted())
                            .child("• Example: !g rust programming, !yt music"),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::SM))
                            .text_color(text::muted())
                            .child("• Just ! with query uses the default provider (DuckDuckGo)"),
                    ),
            )
            .into_any_element();

        // 1 selectable item when there's a query, 0 otherwise
        let count = if has_query { 1 } else { 0 };
        (element, count)
    }

    fn on_select(&self, _index: usize, vx: &ViewContext, _cx: &mut App) -> bool {
        let (provider, search_query) = self.parse_query(vx.query);
        if search_query.is_empty() {
            return false;
        }

        let url = provider
            .url_template
            .replace("{query}", &urlencoding::encode(search_query));
        open_url(&url);
        true // Close launcher
    }

    fn footer_actions(&self, vx: &ViewContext) -> Vec<(&'static str, &'static str)> {
        let (_, search_query) = self.parse_query(vx.query);
        if search_query.is_empty() {
            vec![("Close", "Esc")]
        } else {
            vec![("Search", "Enter"), ("Close", "Esc")]
        }
    }
}

/// Open a URL in the default browser.
fn open_url(url: &str) {
    let url = url.to_string();
    std::thread::spawn(move || {
        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
    });
}

/// URL encoding helper module.
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut encoded = String::new();
        for c in s.chars() {
            match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                    encoded.push(c);
                }
                ' ' => encoded.push_str("%20"),
                _ => {
                    for byte in c.to_string().as_bytes() {
                        encoded.push_str(&format!("%{:02X}", byte));
                    }
                }
            }
        }
        encoded
    }
}
