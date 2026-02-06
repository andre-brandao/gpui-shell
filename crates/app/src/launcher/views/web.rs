//! Web search view — search the web with multiple providers.
//!
//! Supports shebangs: `!g query` (Google), `!yt` (YouTube), `!gh` (GitHub), etc.

use gpui::{AnyElement, App, Context, EventEmitter};
use ui::{ActiveTheme, prelude::*};

use crate::launcher::view::{FooterAction, LauncherView, ViewEvent};

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

pub struct WebSearchView {
    query: String,
}

impl EventEmitter<ViewEvent> for WebSearchView {}

impl WebSearchView {
    pub fn new() -> Self {
        Self {
            query: String::new(),
        }
    }

    fn parse_provider(query: &str) -> (&'static SearchProvider, &str) {
        let query = query.trim();
        for provider in PROVIDERS {
            if let Some(rest) = query
                .strip_prefix(provider.shebang)
                .filter(|r| r.is_empty() || r.starts_with(' '))
            {
                return (provider, rest.trim());
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
    fn id(&self) -> &'static str {
        "web"
    }

    fn prefix(&self) -> &'static str {
        "!"
    }

    fn name(&self) -> &'static str {
        "Web Search"
    }

    fn icon(&self) -> IconName {
        IconName::ToolWeb
    }

    fn description(&self) -> &'static str {
        "Search the web (!g, !yt, !gh, !nix, !ddg)"
    }

    fn match_count(&self) -> usize {
        let (_, search_query) = Self::parse_provider(&self.query);
        if search_query.is_empty() { 0 } else { 1 }
    }

    fn set_query(&mut self, query: &str, _cx: &mut Context<Self>) {
        self.query = query.to_string();
    }

    fn render_item(&self, _index: usize, _selected: bool, _cx: &App) -> AnyElement {
        gpui::Empty.into_any_element()
    }

    fn render_content(&self, cx: &App) -> Option<AnyElement> {
        let colors = cx.theme().colors();
        let (provider, search_query) = Self::parse_provider(&self.query);
        let has_query = !search_query.is_empty();
        let active_shebang = provider.shebang;

        Some(
            div()
                .w_full()
                .p(px(12.))
                .flex()
                .flex_col()
                .gap(px(12.))
                // Search preview card
                .child(
                    div()
                        .w_full()
                        .p(px(12.))
                        .bg(colors.surface_background)
                        .rounded(px(8.))
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        // Provider header
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(
                                            Label::new(provider.icon)
                                                .size(LabelSize::Large)
                                                .color(Color::Default),
                                        )
                                        .child(
                                            Label::new(provider.name)
                                                .size(LabelSize::Default)
                                                .color(Color::Default),
                                        )
                                        .child(
                                            div()
                                                .px(px(6.))
                                                .py(px(2.))
                                                .rounded(px(4.))
                                                .bg(colors.element_background)
                                                .child(
                                                    Label::new(format!("!{}", provider.shebang))
                                                        .size(LabelSize::XSmall)
                                                        .color(Color::Muted),
                                                ),
                                        ),
                                )
                                .when(has_query, |el| {
                                    el.child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.))
                                            .px(px(8.))
                                            .py(px(4.))
                                            .rounded(px(6.))
                                            .bg(colors.ghost_element_selected)
                                            .child(Label::new("Search").size(LabelSize::Small))
                                            .child(
                                                div()
                                                    .px(px(4.))
                                                    .py(px(2.))
                                                    .rounded(px(3.))
                                                    .bg(colors.element_background)
                                                    .child(
                                                        Label::new("Enter")
                                                            .size(LabelSize::XSmall)
                                                            .color(Color::Muted),
                                                    ),
                                            ),
                                    )
                                }),
                        )
                        // Query display
                        .child(
                            div()
                                .w_full()
                                .p(px(8.))
                                .bg(colors.editor_background)
                                .rounded(px(6.))
                                .text_size(px(14.))
                                .child(if has_query {
                                    Label::new(format!("\"{}\"", search_query))
                                        .color(Color::Default)
                                } else {
                                    Label::new("Type your search query...")
                                        .color(Color::Placeholder)
                                }),
                        ),
                )
                // Provider chips
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(
                            Label::new("AVAILABLE PROVIDERS")
                                .size(LabelSize::XSmall)
                                .color(Color::Disabled),
                        )
                        .child(div().flex().flex_wrap().gap(px(6.)).children(
                            PROVIDERS.iter().map(|p| {
                                let is_active = p.shebang == active_shebang;
                                div()
                                    .px(px(8.))
                                    .py(px(4.))
                                    .rounded(px(6.))
                                    .when(is_active, |el| el.bg(colors.ghost_element_selected))
                                    .when(!is_active, |el| el.bg(colors.element_background))
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
                // Usage tips
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(
                            Label::new("USAGE")
                                .size(LabelSize::XSmall)
                                .color(Color::Disabled),
                        )
                        .child(
                            Label::new(
                                "\u{2022} Type !<shebang> <query> to search specific provider",
                            )
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                        )
                        .child(
                            Label::new("\u{2022} Example: !g rust programming, !yt music")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                        .child(
                            Label::new(
                                "\u{2022} Just ! with query uses the default provider (DuckDuckGo)",
                            )
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                        ),
                )
                .into_any_element(),
        )
    }

    fn confirm(&mut self, _index: usize, cx: &mut Context<Self>) {
        let (provider, search_query) = Self::parse_provider(&self.query);
        if search_query.is_empty() {
            return;
        }
        let url = provider
            .url_template
            .replace("{query}", &url_encode(search_query));
        open_url(&url);
        cx.emit(ViewEvent::Close);
    }

    fn footer_actions(&self) -> Vec<FooterAction> {
        let (_, search_query) = Self::parse_provider(&self.query);
        if search_query.is_empty() {
            vec![FooterAction {
                label: "Close",
                key: "Esc",
            }]
        } else {
            vec![
                FooterAction {
                    label: "Search",
                    key: "Enter",
                },
                FooterAction {
                    label: "Close",
                    key: "Esc",
                },
            ]
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
