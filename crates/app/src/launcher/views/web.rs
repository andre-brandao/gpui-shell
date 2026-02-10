//! Web search view for searching various web providers.
//!
//! Supports multiple search providers via "shebangs":
//! - `!g query` - Google
//! - `!yt query` - YouTube
//! - `!gh query` - GitHub
//! - `!nix query` - Nixpkgs
//! - `!ddg query` - DuckDuckGo (default)
//! - `! query` - Default provider

use gpui::{AnyElement, App, div, prelude::*, px, rgba};
use ui::{ActiveTheme, Color, Label, LabelCommon, LabelSize, font_size, radius, spacing};

use crate::launcher::view::{LauncherView, ViewContext};

/// A search provider with its shebang and URL template.
struct SearchProvider {
    shebang: &'static str,
    name: &'static str,
    icon: &'static str,
    url_template: &'static str,
    is_default: bool,
}

const PROVIDERS: &[SearchProvider] = &[
    SearchProvider {
        shebang: "g",
        name: "Google",
        icon: "\u{f1a0}",
        url_template: "https://www.google.com/search?q={query}",
        is_default: false,
    },
    SearchProvider {
        shebang: "yt",
        name: "YouTube",
        icon: "\u{f167}",
        url_template: "https://www.youtube.com/results?search_query={query}",
        is_default: false,
    },
    SearchProvider {
        shebang: "gh",
        name: "GitHub",
        icon: "\u{f09b}",
        url_template: "https://github.com/search?q={query}",
        is_default: false,
    },
    SearchProvider {
        shebang: "nix",
        name: "Nixpkgs",
        icon: "\u{f313}",
        url_template: "https://search.nixos.org/packages?query={query}",
        is_default: false,
    },
    SearchProvider {
        shebang: "ddg",
        name: "DuckDuckGo",
        icon: "\u{f1a5}",
        url_template: "https://duckduckgo.com/?q={query}",
        is_default: true,
    },
    SearchProvider {
        shebang: "w",
        name: "Wikipedia",
        icon: "\u{f266}",
        url_template: "https://en.wikipedia.org/wiki/Special:Search?search={query}",
        is_default: false,
    },
    SearchProvider {
        shebang: "rs",
        name: "crates.io",
        icon: "\u{e7a8}",
        url_template: "https://crates.io/search?q={query}",
        is_default: false,
    },
    SearchProvider {
        shebang: "r",
        name: "Reddit",
        icon: "\u{f281}",
        url_template: "https://www.reddit.com/search?q={query}",
        is_default: false,
    },
];

/// Web search view - search the web with various providers.
pub struct WebSearchView;

impl WebSearchView {
    /// Parse the query to extract provider and search terms.
    fn parse_query<'a>(&self, query: &'a str) -> (&'static SearchProvider, &'a str) {
        let query = query.trim();

        for provider in PROVIDERS {
            let prefix = provider.shebang;
            if query.starts_with(prefix) {
                let rest = &query[prefix.len()..];
                if rest.is_empty() || rest.starts_with(' ') {
                    return (provider, rest.trim());
                }
            }
        }

        let default = PROVIDERS
            .iter()
            .find(|p| p.is_default)
            .unwrap_or(&PROVIDERS[0]);
        (default, query)
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

    fn match_count(&self, vx: &ViewContext, _cx: &App) -> usize {
        let (_, search_query) = self.parse_query(vx.query);
        if search_query.is_empty() { 0 } else { 1 }
    }

    fn render_item(
        &self,
        _index: usize,
        _selected: bool,
        _vx: &ViewContext,
        _cx: &App,
    ) -> AnyElement {
        div().into_any_element()
    }

    fn render_content(&self, vx: &ViewContext, cx: &App) -> Option<AnyElement> {
        let theme = cx.theme();
        let (provider, search_query) = self.parse_query(vx.query);
        let has_query = !search_query.is_empty();

        let bg_secondary = theme.bg.secondary;
        let interactive_default = theme.interactive.default;
        let accent_selection = theme.accent.selection;
        let interactive_hover = theme.interactive.hover;

        Some(
            div()
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
                        .bg(bg_secondary)
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
                                            Label::new(provider.icon)
                                                .size(LabelSize::Large)
                                                .color(Color::Default),
                                        )
                                        .child(Label::new(provider.name).size(LabelSize::Default))
                                        .child(
                                            div()
                                                .px(px(6.))
                                                .py(px(2.))
                                                .rounded(px(4.))
                                                .bg(interactive_default)
                                                .child(
                                                    Label::new(format!("!{}", provider.shebang))
                                                        .size(LabelSize::XSmall)
                                                        .color(Color::Muted),
                                                ),
                                        ),
                                )
                                // Search hint
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(spacing::SM))
                                        .px(px(spacing::SM))
                                        .py(px(4.))
                                        .rounded(px(radius::SM))
                                        .when(has_query && vx.selected_index == 0, move |el| {
                                            el.bg(accent_selection)
                                        })
                                        .when(has_query && vx.selected_index != 0, move |el| {
                                            el.bg(interactive_hover)
                                        })
                                        .when(!has_query, |el| el.bg(rgba(0x00000033)))
                                        .child(if has_query {
                                            Label::new("Search").size(LabelSize::Small)
                                        } else {
                                            Label::new("Search")
                                                .size(LabelSize::Small)
                                                .color(Color::Disabled)
                                        })
                                        .child(
                                            div()
                                                .px(px(4.))
                                                .py(px(2.))
                                                .rounded(px(3.))
                                                .bg(rgba(0x00000044))
                                                .child(if has_query {
                                                    Label::new("Enter")
                                                        .size(LabelSize::XSmall)
                                                        .color(Color::Muted)
                                                } else {
                                                    Label::new("Enter")
                                                        .size(LabelSize::XSmall)
                                                        .color(Color::Disabled)
                                                }),
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
                                .child(if has_query {
                                    Label::new(format!("\"{}\"", search_query))
                                        .color(Color::Default)
                                } else {
                                    Label::new("Type your search query...")
                                        .color(Color::Placeholder)
                                }),
                        ),
                )
                // Provider list
                .child(
                    div()
                        .w_full()
                        .pt(px(spacing::SM))
                        .flex()
                        .flex_col()
                        .gap(px(spacing::XS))
                        .child(
                            Label::new("AVAILABLE PROVIDERS")
                                .size(LabelSize::XSmall)
                                .color(Color::Disabled),
                        )
                        .child(div().flex().flex_wrap().gap(px(spacing::SM)).children(
                            PROVIDERS.iter().map(|p| {
                                let is_active = p.shebang == provider.shebang;
                                div()
                                    .px(px(spacing::SM))
                                    .py(px(4.))
                                    .rounded(px(radius::SM))
                                    .when(is_active, move |el| el.bg(accent_selection))
                                    .when(!is_active, move |el| el.bg(interactive_default))
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .child(
                                        Label::new(p.icon)
                                            .size(LabelSize::Small)
                                            .color(Color::Default),
                                    )
                                    .child(
                                        Label::new(format!("!{}", p.shebang))
                                            .size(LabelSize::Small)
                                            .color(Color::Muted),
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
                            Label::new("USAGE")
                                .size(LabelSize::XSmall)
                                .color(Color::Disabled),
                        )
                        .child(
                            Label::new("• Type !<shebang> <query> to search specific provider")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                        .child(
                            Label::new("• Example: !g rust programming, !yt music")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                        .child(
                            Label::new(
                                "• Just ! with query uses the default provider (DuckDuckGo)",
                            )
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                        ),
                )
                .into_any_element(),
        )
    }

    fn on_select(&self, _index: usize, vx: &ViewContext, _cx: &mut App) -> bool {
        let (provider, search_query) = self.parse_query(vx.query);
        if search_query.is_empty() {
            return false;
        }

        let url = provider
            .url_template
            .replace("{query}", &url_encode(search_query));
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

fn url_encode(s: &str) -> String {
    let mut encoded = String::new();
    for c in s.chars() {
        match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => encoded.push(c),
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

fn open_url(url: &str) {
    let url = url.to_string();
    std::thread::spawn(move || {
        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
    });
}
