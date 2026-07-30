//! Web search view for searching various web providers.

pub mod config;

use gpui::{AnyElement, App, div, prelude::*, px, rgba};
use ui::{
    ActiveTheme, Color, Icon, IconName, IconSize, Label, LabelCommon, Radius, Spacing, TextSize,
};

use self::config::{WebConfig, WebProviderConfig};
use ui::patterns::footer_hints;

use crate::launcher::view::{LauncherView, ViewContext};

/// Web search view - search the web with various providers.
pub struct WebSearchView {
    prefix: String,
    providers: Vec<WebProviderConfig>,
}

impl WebSearchView {
    pub fn new(config: &WebConfig) -> Self {
        Self {
            prefix: config.prefix.clone(),
            providers: config.providers.clone(),
        }
    }

    fn parse_query<'a>(&'a self, query: &'a str) -> (&'a WebProviderConfig, &'a str) {
        let query = query.trim();

        for provider in &self.providers {
            let prefix = &provider.shebang;
            if let Some(rest) = query.strip_prefix(prefix.as_str())
                && (rest.is_empty() || rest.starts_with(' '))
            {
                return (provider, rest.trim());
            }
        }

        let default = self
            .providers
            .iter()
            .find(|p| p.default)
            .or_else(|| self.providers.first())
            .expect("at least one provider must exist");
        (default, query)
    }
}

impl LauncherView for WebSearchView {
    fn prefix(&self) -> &str {
        &self.prefix
    }

    fn name(&self) -> &'static str {
        "Web Search"
    }

    fn icon(&self) -> IconName {
        IconName::Globe
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

        let bg_secondary = theme.colors.surface_background;
        let interactive_default = theme.colors.element_background;
        let accent_selection = theme.colors.element_selected;
        let interactive_hover = theme.colors.element_hover;

        let provider_icon = provider.icon();
        let provider_name = provider.name.clone();
        let provider_shebang = provider.shebang.clone();

        Some(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(Spacing::Large.pixels())
                .p(Spacing::Large.pixels())
                .child(
                    div()
                        .w_full()
                        .p(Spacing::Large.pixels())
                        .bg(bg_secondary)
                        .rounded(Radius::Medium.pixels())
                        .flex()
                        .flex_col()
                        .gap(Spacing::Medium.pixels())
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(Spacing::Medium.pixels())
                                        .child(Icon::new(provider_icon).size(IconSize::Large))
                                        .child(Label::new(provider_name).size(TextSize::Default))
                                        .child(
                                            div()
                                                .px(px(6.))
                                                .py(px(2.))
                                                .rounded(px(4.))
                                                .bg(interactive_default)
                                                .child(
                                                    Label::new(format!("!{}", provider_shebang))
                                                        .size(TextSize::XSmall)
                                                        .color(Color::Muted),
                                                ),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(Spacing::Medium.pixels())
                                        .px(Spacing::Medium.pixels())
                                        .py(px(4.))
                                        .rounded(Radius::Small.pixels())
                                        .when(has_query && vx.selected_index == 0, move |el| {
                                            el.bg(accent_selection)
                                        })
                                        .when(has_query && vx.selected_index != 0, move |el| {
                                            el.bg(interactive_hover)
                                        })
                                        .when(!has_query, |el| el.bg(rgba(0x00000033)))
                                        .child(if has_query {
                                            Label::new("Search").size(TextSize::Small)
                                        } else {
                                            Label::new("Search")
                                                .size(TextSize::Small)
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
                                                        .size(TextSize::XSmall)
                                                        .color(Color::Muted)
                                                } else {
                                                    Label::new("Enter")
                                                        .size(TextSize::XSmall)
                                                        .color(Color::Disabled)
                                                }),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .w_full()
                                .p(Spacing::Medium.pixels())
                                .bg(rgba(0x00000066))
                                .rounded(Radius::Small.pixels())
                                .text_size(TextSize::Default.rems())
                                .child(if has_query {
                                    Label::new(format!("\"{}\"", search_query))
                                        .color(Color::Default)
                                        .size(TextSize::Default)
                                } else {
                                    Label::new("Type your search query...")
                                        .color(Color::Placeholder)
                                        .size(TextSize::Default)
                                }),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .pt(Spacing::Medium.pixels())
                        .flex()
                        .flex_col()
                        .gap(Spacing::XSmall.pixels())
                        .child(
                            Label::new("AVAILABLE PROVIDERS")
                                .size(TextSize::XSmall)
                                .color(Color::Disabled),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap(Spacing::Medium.pixels())
                                .children(self.providers.iter().map(|p| {
                                    let is_active = p.shebang == provider.shebang;
                                    let icon = p.icon();
                                    let shebang = p.shebang.clone();
                                    div()
                                        .px(Spacing::Medium.pixels())
                                        .py(px(4.))
                                        .rounded(Radius::Small.pixels())
                                        .when(is_active, move |el| el.bg(accent_selection))
                                        .when(!is_active, move |el| el.bg(interactive_default))
                                        .flex()
                                        .items_center()
                                        .gap(px(4.))
                                        .child(Icon::new(icon).size(IconSize::Small))
                                        .child(
                                            Label::new(format!("!{}", shebang))
                                                .size(TextSize::Small)
                                                .color(Color::Muted),
                                        )
                                })),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .pt(Spacing::Large.pixels())
                        .flex()
                        .flex_col()
                        .gap(Spacing::XSmall.pixels())
                        .child(
                            Label::new("USAGE")
                                .size(TextSize::XSmall)
                                .color(Color::Disabled),
                        )
                        .child(
                            Label::new("• Type !<shebang> <query> to search specific provider")
                                .size(TextSize::Small)
                                .color(Color::Muted),
                        )
                        .child(
                            Label::new("• Example: !g rust programming, !yt music")
                                .size(TextSize::Small)
                                .color(Color::Muted),
                        )
                        .child(
                            Label::new(
                                "• Just ! with query uses the default provider (DuckDuckGo)",
                            )
                            .size(TextSize::Small)
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

        let url = provider.url.replace("{query}", &url_encode(search_query));
        open_url(&url);
        true
    }

    fn render_footer_bar(&self, vx: &ViewContext, cx: &App) -> AnyElement {
        let (_, search_query) = self.parse_query(vx.query);
        let actions = if search_query.is_empty() {
            vec![("Close", "Esc")]
        } else {
            vec![("Search", "Enter"), ("Close", "Esc")]
        };
        footer_hints(actions, cx)
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
