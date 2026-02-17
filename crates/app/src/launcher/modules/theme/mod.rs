//! Theme browser view for the launcher.

pub mod config;

use std::sync::Mutex;

use gpui::{AnyElement, App, FontWeight, div, prelude::*, px};
use services::{THEME_PROVIDERS, ThemeProvider, ThemeRepository, load_stylix_scheme};
use ui::{
    ActiveTheme, Base16Colors, Theme, ThemeScheme, builtin_schemes, font_size, radius, spacing,
};

use self::config::ThemesConfig;
use crate::config::Config;
use crate::launcher::view::{LauncherView, ViewContext, render_footer_hints};

const MAX_VISIBLE_THEMES: usize = 50;

static CACHED_SCHEMES: Mutex<Option<Vec<ThemeScheme>>> = Mutex::new(None);

/// Launcher view for browsing and applying themes.
pub struct ThemeView {
    prefix: String,
}

impl ThemeView {
    pub fn new(config: &ThemesConfig) -> Self {
        Self {
            prefix: config.prefix.clone(),
        }
    }

    fn visible_schemes(query: &str) -> Vec<ThemeScheme> {
        all_schemes(query)
            .into_iter()
            .take(MAX_VISIBLE_THEMES)
            .collect()
    }
}

impl LauncherView for ThemeView {
    fn prefix(&self) -> &str {
        &self.prefix
    }

    fn name(&self) -> &'static str {
        "Themes"
    }

    fn icon(&self) -> &'static str {
        ""
    }

    fn description(&self) -> &'static str {
        "Browse and apply themes"
    }

    fn match_count(&self, vx: &ViewContext, _cx: &App) -> usize {
        Self::visible_schemes(vx.query).len()
    }

    fn render_header(&self, _vx: &ViewContext, cx: &App) -> Option<AnyElement> {
        let theme = cx.theme();
        let current_accent = theme.accent.primary;
        let current_bg = theme.bg.primary;

        let mut header = div()
            .flex()
            .flex_col()
            .gap(px(spacing::SM))
            .p(px(spacing::SM));

        if let Some(stylix) = stylix_scheme() {
            let is_active = colors_match(stylix.theme.accent.primary, current_accent)
                && colors_match(stylix.theme.bg.primary, current_bg);
            header = header.child(render_stylix_card(&stylix, is_active, theme));
        }

        for provider in THEME_PROVIDERS {
            let repo = ThemeRepository::new(provider);
            header = header.child(render_provider_card(provider, repo.is_cached(), theme));
        }

        Some(header.into_any_element())
    }

    fn render_item(&self, index: usize, selected: bool, vx: &ViewContext, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let schemes = Self::visible_schemes(vx.query);
        let current_accent = theme.accent.primary;
        let current_bg = theme.bg.primary;

        if let Some(scheme) = schemes.get(index) {
            let is_active = colors_match(scheme.theme.accent.primary, current_accent)
                && colors_match(scheme.theme.bg.primary, current_bg);
            render_theme_card(scheme, selected, is_active, theme)
        } else {
            div().into_any_element()
        }
    }

    fn on_select(&self, index: usize, vx: &ViewContext, cx: &mut App) -> bool {
        let schemes = Self::visible_schemes(vx.query);
        if let Some(scheme) = schemes.get(index) {
            Theme::set(scheme.theme.clone(), cx);
            if let Err(err) = Config::save_theme(cx) {
                tracing::warn!("Failed to persist selected theme: {}", err);
            }
        }
        false
    }

    fn render_footer_bar(&self, _vx: &ViewContext, cx: &App) -> AnyElement {
        render_footer_hints(vec![("Apply", "Enter"), ("Close", "Esc")], cx)
    }
}

fn stylix_scheme() -> Option<ThemeScheme> {
    let b16 = load_stylix_scheme()?;
    let p = &b16.palette;
    let colors = Base16Colors::from_hex(&[
        &p.base00, &p.base01, &p.base02, &p.base03, &p.base04, &p.base05, &p.base06, &p.base07,
        &p.base08, &p.base09, &p.base0a, &p.base0b, &p.base0c, &p.base0d, &p.base0e, &p.base0f,
    ])
    .ok()?;

    Some(ThemeScheme {
        name: Box::leak(b16.name.into_boxed_str()),
        description: Box::leak(format!("Stylix — {}", b16.author).into_boxed_str()),
        theme: colors.to_theme(),
    })
}

fn build_schemes() -> Vec<ThemeScheme> {
    let mut schemes = builtin_schemes();

    for provider in THEME_PROVIDERS {
        let repo = ThemeRepository::new(provider);
        for b16 in repo.load_cached() {
            let p = &b16.palette;
            let colors = match Base16Colors::from_hex(&[
                &p.base00, &p.base01, &p.base02, &p.base03, &p.base04, &p.base05, &p.base06,
                &p.base07, &p.base08, &p.base09, &p.base0a, &p.base0b, &p.base0c, &p.base0d,
                &p.base0e, &p.base0f,
            ]) {
                Ok(c) => c,
                Err(_) => continue,
            };

            schemes.push(ThemeScheme {
                name: Box::leak(b16.name.into_boxed_str()),
                description: Box::leak(
                    format!("{} — {}", provider.name, b16.author).into_boxed_str(),
                ),
                theme: colors.to_theme(),
            });
        }
    }

    schemes
}

fn invalidate_schemes_cache() {
    let mut cache = CACHED_SCHEMES.lock().unwrap();
    *cache = None;
}

fn all_schemes(query: &str) -> Vec<ThemeScheme> {
    let mut cache = CACHED_SCHEMES.lock().unwrap();
    let schemes = cache.get_or_insert_with(build_schemes);

    if query.is_empty() {
        return schemes.clone();
    }

    let query_lower = query.to_lowercase();
    schemes
        .iter()
        .filter(|s| {
            s.name.to_lowercase().contains(&query_lower)
                || s.description.to_lowercase().contains(&query_lower)
        })
        .cloned()
        .collect()
}

fn colors_match(a: gpui::Hsla, b: gpui::Hsla) -> bool {
    (a.h - b.h).abs() < 0.01 && (a.s - b.s).abs() < 0.01 && (a.l - b.l).abs() < 0.01
}

fn render_stylix_card(scheme: &ThemeScheme, is_active: bool, theme: &Theme) -> AnyElement {
    let bg_secondary = theme.bg.secondary;
    let bg_primary = theme.bg.primary;
    let text_primary = theme.text.primary;
    let text_disabled = theme.text.disabled;
    let accent_primary = theme.accent.primary;
    let preview_colors = scheme.preview_colors();
    let stylix_theme = scheme.theme.clone();

    div()
        .id("stylix-card")
        .w_full()
        .p(px(spacing::MD))
        .rounded(px(radius::LG))
        .bg(bg_secondary)
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            Theme::set(stylix_theme.clone(), cx);
            if let Err(err) = Config::save_theme(cx) {
                tracing::warn!("Failed to persist selected theme: {}", err);
            }
        })
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
                        .text_size(px(font_size::LG))
                        .text_color(accent_primary)
                        .child(""),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .child(
                            div()
                                .text_size(px(font_size::MD))
                                .text_color(text_primary)
                                .font_weight(FontWeight::MEDIUM)
                                .child(scheme.name),
                        )
                        .child(
                            div()
                                .text_size(px(font_size::XS))
                                .text_color(text_disabled)
                                .child(scheme.description),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::SM))
                .child(render_color_strip(&preview_colors))
                .when(is_active, move |el| {
                    el.child(
                        div()
                            .px(px(spacing::SM))
                            .py(px(2.))
                            .rounded(px(radius::SM))
                            .bg(accent_primary)
                            .text_size(px(font_size::XS))
                            .text_color(bg_primary)
                            .font_weight(FontWeight::BOLD)
                            .child("Active"),
                    )
                }),
        )
        .into_any_element()
}

fn render_provider_card(
    provider: &'static ThemeProvider,
    is_downloaded: bool,
    theme: &Theme,
) -> AnyElement {
    let bg_secondary = theme.bg.secondary;
    let text_primary = theme.text.primary;
    let text_disabled = theme.text.disabled;
    let accent_primary = theme.accent.primary;
    let interactive_hover = theme.interactive.hover;

    let (icon, action) = if is_downloaded {
        ("󰚰", format!("Update {}", provider.name))
    } else {
        ("󰇚", format!("Download {}", provider.name))
    };

    let provider_id = provider.id;

    div()
        .id(format!("provider-{}", provider_id))
        .w_full()
        .px(px(spacing::MD))
        .py(px(spacing::SM))
        .rounded(px(radius::LG))
        .bg(bg_secondary)
        .cursor_pointer()
        .hover(move |s| s.bg(interactive_hover))
        .on_click(move |_, _, _cx| {
            let provider = THEME_PROVIDERS.iter().find(|p| p.id == provider_id);
            if let Some(provider) = provider {
                let repo = ThemeRepository::new(provider);
                match repo.fetch_and_cache() {
                    Ok(schemes) => {
                        tracing::info!("Fetched {} schemes from {}", schemes.len(), provider.name);
                        invalidate_schemes_cache();
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch from {}: {}", provider.name, e);
                    }
                }
            }
        })
        .flex()
        .items_center()
        .gap(px(spacing::SM))
        .child(
            div()
                .text_size(px(font_size::LG))
                .text_color(accent_primary)
                .child(icon),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(1.))
                .child(
                    div()
                        .text_size(px(font_size::SM))
                        .text_color(text_primary)
                        .font_weight(FontWeight::MEDIUM)
                        .child(action),
                )
                .child(
                    div()
                        .text_size(px(font_size::XS))
                        .text_color(text_disabled)
                        .child(provider.repo),
                ),
        )
        .into_any_element()
}

fn render_theme_card(
    scheme: &ThemeScheme,
    is_selected: bool,
    is_active: bool,
    theme: &Theme,
) -> AnyElement {
    let accent_selection = theme.accent.selection;
    let interactive_hover = theme.interactive.hover;
    let bg_primary = theme.bg.primary;
    let text_primary = theme.text.primary;
    let text_disabled = theme.text.disabled;
    let border_default = theme.border.default;
    let accent_primary = theme.accent.primary;

    let preview_colors = scheme.preview_colors();
    let name = scheme.name;
    let description = scheme.description;
    let card_theme = scheme.theme.clone();

    div()
        .id(format!("theme-{}", name))
        .w_full()
        .px(px(spacing::MD))
        .py(px(spacing::SM))
        .rounded(px(radius::LG))
        .border_1()
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            Theme::set(card_theme.clone(), cx);
            if let Err(err) = Config::save_theme(cx) {
                tracing::warn!("Failed to persist selected theme: {}", err);
            }
        })
        .when(is_selected, move |el| {
            el.bg(accent_selection).border_color(accent_primary)
        })
        .when(!is_selected, move |el| {
            el.border_color(border_default)
                .hover(move |s| s.bg(interactive_hover))
        })
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(1.))
                .child(
                    div()
                        .text_size(px(font_size::SM))
                        .text_color(text_primary)
                        .font_weight(FontWeight::MEDIUM)
                        .child(name),
                )
                .child(
                    div()
                        .text_size(px(font_size::XS))
                        .text_color(text_disabled)
                        .child(description),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::SM))
                .child(render_color_strip(&preview_colors))
                .when(is_active, move |el| {
                    el.child(
                        div()
                            .px(px(spacing::SM))
                            .py(px(2.))
                            .rounded(px(radius::SM))
                            .bg(accent_primary)
                            .text_size(px(font_size::XS))
                            .text_color(bg_primary)
                            .font_weight(FontWeight::BOLD)
                            .child("Active"),
                    )
                }),
        )
        .into_any_element()
}

fn render_color_strip(colors: &[gpui::Hsla]) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(2.))
        .children(
            colors
                .iter()
                .map(|&color| div().w(px(14.)).h(px(14.)).rounded(px(3.)).bg(color)),
        )
        .into_any_element()
}
