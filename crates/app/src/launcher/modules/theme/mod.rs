//! Theme browser view for the launcher.

pub mod config;

use std::sync::Mutex;

use gpui::{AnyElement, App, FontWeight, div, prelude::*, px};
use services::{THEME_PROVIDERS, ThemeProvider, ThemeRepository, load_stylix_scheme};
use ui::{
    ActiveTheme, Base16Palette, Color, Icon, IconName, IconSize, Radius, Spacing, TextSize, Theme,
    ThemeScheme, builtin_schemes,
};

use self::config::ThemesConfig;
use crate::config::Config;
use ui::patterns::footer_hints;

use crate::launcher::view::{LauncherView, ViewContext};

const MAX_VISIBLE_THEMES: usize = 50;
const THEME_ICON: IconName = IconName::Palette;

static CACHED_SCHEMES: Mutex<Option<Vec<ThemeScheme>>> = Mutex::new(None);

/// Launcher view for browsing and applying themes.
pub struct ThemeView {
    prefix: String,
    /// Schemes matching the current query, refreshed once per frame by
    /// `update_matches`.
    matches: Vec<ThemeScheme>,
}

impl ThemeView {
    pub fn new(config: &ThemesConfig) -> Self {
        Self {
            prefix: config.prefix.clone(),
            matches: Vec::new(),
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

    fn icon(&self) -> IconName {
        THEME_ICON
    }

    fn description(&self) -> &'static str {
        "Browse and apply themes"
    }

    fn update_matches(&mut self, vx: &ViewContext, _cx: &App) {
        self.matches = Self::visible_schemes(vx.query);
    }

    fn match_count(&self, _vx: &ViewContext, _cx: &App) -> usize {
        self.matches.len()
    }

    fn render_header(&self, _vx: &ViewContext, cx: &App) -> Option<AnyElement> {
        let theme = cx.theme();
        let current_accent = theme.colors.accent;
        let current_bg = theme.colors.background;

        let mut header = div()
            .flex()
            .flex_col()
            .gap(Spacing::Medium.pixels())
            .p(Spacing::Medium.pixels());

        if let Some(stylix) = stylix_scheme() {
            let is_active = colors_match(stylix.palette.base0d, current_accent)
                && colors_match(stylix.palette.base00, current_bg);
            header = header.child(render_stylix_card(&stylix, is_active, theme));
        }

        for provider in THEME_PROVIDERS {
            let repo = ThemeRepository::new(provider);
            header = header.child(render_provider_card(provider, repo.is_cached(), theme));
        }

        Some(header.into_any_element())
    }

    fn render_item(&self, index: usize, selected: bool, _vx: &ViewContext, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let schemes = &self.matches;
        let current_accent = theme.colors.accent;
        let current_bg = theme.colors.background;

        if let Some(scheme) = schemes.get(index) {
            let is_active = colors_match(scheme.palette.base0d, current_accent)
                && colors_match(scheme.palette.base00, current_bg);
            render_theme_card(scheme, selected, is_active, theme)
        } else {
            div().into_any_element()
        }
    }

    fn on_select(&self, index: usize, _vx: &ViewContext, cx: &mut App) -> bool {
        if let Some(scheme) = self.matches.get(index) {
            // Swapping only the palette keeps the user's font size and any
            // token overrides intact.
            let mut new_theme = cx.theme().clone();
            new_theme.set_palette(scheme.name.clone(), scheme.palette);

            Theme::set(new_theme, cx);
            if let Err(err) = Config::save_theme(cx) {
                tracing::warn!("Failed to persist selected theme: {}", err);
            }
        }
        false
    }

    fn render_footer_bar(&self, _vx: &ViewContext, cx: &App) -> AnyElement {
        footer_hints(vec![("Apply", "Enter"), ("Close", "Esc")], cx)
    }
}

fn stylix_scheme() -> Option<ThemeScheme> {
    let b16 = load_stylix_scheme()?;
    let p = &b16.palette;
    let palette = Base16Palette::from_hex(&[
        &p.base00, &p.base01, &p.base02, &p.base03, &p.base04, &p.base05, &p.base06, &p.base07,
        &p.base08, &p.base09, &p.base0a, &p.base0b, &p.base0c, &p.base0d, &p.base0e, &p.base0f,
    ])
    .ok()?;

    Some(ThemeScheme::new(
        b16.name,
        format!("Stylix — {}", b16.author),
        palette,
    ))
}

fn build_schemes() -> Vec<ThemeScheme> {
    let mut schemes = builtin_schemes();

    for provider in THEME_PROVIDERS {
        let repo = ThemeRepository::new(provider);
        for b16 in repo.load_cached() {
            let p = &b16.palette;
            let palette = match Base16Palette::from_hex(&[
                &p.base00, &p.base01, &p.base02, &p.base03, &p.base04, &p.base05, &p.base06,
                &p.base07, &p.base08, &p.base09, &p.base0a, &p.base0b, &p.base0c, &p.base0d,
                &p.base0e, &p.base0f,
            ]) {
                Ok(c) => c,
                Err(_) => continue,
            };

            schemes.push(ThemeScheme::new(
                b16.name,
                format!("{} — {}", provider.name, b16.author),
                palette,
            ));
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
    let bg_secondary = theme.colors.surface_background;
    let bg_primary = theme.colors.background;
    let text_primary = theme.colors.text;
    let text_disabled = theme.colors.text_disabled;
    let accent_primary = theme.colors.accent;
    let preview_colors = scheme.preview_colors();
    let stylix_theme = (scheme.name.clone(), scheme.palette);

    div()
        .id("stylix-card")
        .w_full()
        .p(Spacing::Large.pixels())
        .rounded(Radius::Large.pixels())
        .bg(bg_secondary)
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            let (name, palette) = stylix_theme.clone();
            let mut new_theme = cx.theme().clone();
            new_theme.set_palette(name, palette);
            Theme::set(new_theme, cx);
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
                .gap(Spacing::Medium.pixels())
                .child(
                    Icon::new(THEME_ICON)
                        .size(IconSize::Large)
                        .color(Color::Accent),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .child(
                            div()
                                .text_size(TextSize::Medium.rems())
                                .text_color(text_primary)
                                .font_weight(FontWeight::MEDIUM)
                                .child(scheme.name.clone()),
                        )
                        .child(
                            div()
                                .text_size(TextSize::XSmall.rems())
                                .text_color(text_disabled)
                                .child(scheme.description.clone()),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(Spacing::Medium.pixels())
                .child(render_color_strip(&preview_colors))
                .when(is_active, move |el| {
                    el.child(
                        div()
                            .px(Spacing::Medium.pixels())
                            .py(px(2.))
                            .rounded(Radius::Small.pixels())
                            .bg(accent_primary)
                            .text_size(TextSize::XSmall.rems())
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
    let bg_secondary = theme.colors.surface_background;
    let text_primary = theme.colors.text;
    let text_disabled = theme.colors.text_disabled;
    let interactive_hover = theme.colors.element_hover;

    let (icon, action) = if is_downloaded {
        (IconName::Refresh, format!("Update {}", provider.name))
    } else {
        (IconName::Download, format!("Download {}", provider.name))
    };

    let provider_id = provider.id;

    div()
        .id(format!("provider-{}", provider_id))
        .w_full()
        .px(Spacing::Large.pixels())
        .py(Spacing::Medium.pixels())
        .rounded(Radius::Large.pixels())
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
        .gap(Spacing::Medium.pixels())
        .child(Icon::new(icon).size(IconSize::Large).color(Color::Accent))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(1.))
                .child(
                    div()
                        .text_size(TextSize::Small.rems())
                        .text_color(text_primary)
                        .font_weight(FontWeight::MEDIUM)
                        .child(action),
                )
                .child(
                    div()
                        .text_size(TextSize::XSmall.rems())
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
    let accent_selection = theme.colors.element_selected;
    let interactive_hover = theme.colors.element_hover;
    let bg_primary = theme.colors.background;
    let text_primary = theme.colors.text;
    let text_disabled = theme.colors.text_disabled;
    let border_default = theme.colors.border;
    let accent_primary = theme.colors.accent;

    let preview_colors = scheme.preview_colors();
    let name = scheme.name.clone();
    let description = scheme.description.clone();
    let card_theme = (scheme.name.clone(), scheme.palette);

    div()
        .id(format!("theme-{}", name))
        .w_full()
        .px(Spacing::Large.pixels())
        .py(Spacing::Medium.pixels())
        .rounded(Radius::Large.pixels())
        .border_1()
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            let (name, palette) = card_theme.clone();
            let mut new_theme = cx.theme().clone();
            new_theme.set_palette(name, palette);
            Theme::set(new_theme, cx);
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
                        .text_size(TextSize::Small.rems())
                        .text_color(text_primary)
                        .font_weight(FontWeight::MEDIUM)
                        .child(name),
                )
                .child(
                    div()
                        .text_size(TextSize::XSmall.rems())
                        .text_color(text_disabled)
                        .child(description),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(Spacing::Medium.pixels())
                .child(render_color_strip(&preview_colors))
                .when(is_active, move |el| {
                    el.child(
                        div()
                            .px(Spacing::Medium.pixels())
                            .py(px(2.))
                            .rounded(Radius::Small.pixels())
                            .bg(accent_primary)
                            .text_size(TextSize::XSmall.rems())
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
