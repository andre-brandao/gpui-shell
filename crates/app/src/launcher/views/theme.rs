//! Theme browser view for the launcher.
//!
//! Lists available theme schemes with color preview swatches.
//! Selecting a theme applies it globally. Includes a "Fetch themes"
//! action to download Base16 schemes from a remote repository.

use std::sync::Mutex;

use crate::launcher::view::{LauncherView, ViewContext};
use gpui::{AnyElement, App, FontWeight, div, prelude::*, px};
use services::{ThemeRepository, load_stylix_scheme};
use ui::{
    ActiveTheme, Base16Colors, Theme, ThemeScheme, builtin_schemes, font_size, radius, spacing,
};

/// Cached scheme data. `None` means not yet loaded; populated on first access.
static CACHED_SCHEMES: Mutex<Option<Vec<ThemeScheme>>> = Mutex::new(None);

/// Launcher view for browsing and applying themes.
pub struct ThemeView;

impl LauncherView for ThemeView {
    fn prefix(&self) -> &'static str {
        "~"
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

    fn render(&self, vx: &ViewContext, cx: &App) -> (AnyElement, usize) {
        let theme = cx.theme();
        let stylix = stylix_scheme();
        let schemes = all_schemes(vx.query);

        // Layout: [fetch] [stylix?] [schemes...]
        let has_stylix = stylix.is_some();
        let stylix_offset = if has_stylix { 1 } else { 0 };
        let count = 1 + stylix_offset + schemes.len();

        let current_accent = theme.accent.primary;
        let current_bg = theme.bg.primary;

        let element = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(spacing::SM))
            .p(px(spacing::SM))
            // Fetch action card
            .child(render_fetch_card(vx.selected_index == 0, theme))
            // Stylix card (if available)
            .when_some(stylix, |el, scheme| {
                let is_selected = vx.selected_index == 1;
                let is_active = colors_match(scheme.theme.accent.primary, current_accent)
                    && colors_match(scheme.theme.bg.primary, current_bg);
                el.child(render_stylix_card(&scheme, is_selected, is_active, theme))
            })
            // Theme cards
            .children(schemes.into_iter().enumerate().map(|(i, scheme)| {
                let list_index = i + 1 + stylix_offset;
                let is_selected = list_index == vx.selected_index;
                let is_active = colors_match(scheme.theme.accent.primary, current_accent)
                    && colors_match(scheme.theme.bg.primary, current_bg);
                render_theme_card(&scheme, is_selected, is_active, theme)
            }))
            .into_any_element();

        (element, count)
    }

    fn on_select(&self, index: usize, vx: &ViewContext, cx: &mut App) -> bool {
        if index == 0 {
            // Fetch themes action
            let repo = ThemeRepository::new(None, None);
            match repo.fetch_and_cache() {
                Ok(schemes) => {
                    tracing::info!("Fetched {} Base16 schemes", schemes.len());
                    invalidate_schemes_cache();
                }
                Err(e) => {
                    tracing::error!("Failed to fetch themes: {}", e);
                }
            }
            return false;
        }

        let stylix = stylix_scheme();
        let has_stylix = stylix.is_some();
        let stylix_offset = if has_stylix { 1 } else { 0 };

        if has_stylix && index == 1 {
            if let Some(scheme) = stylix {
                Theme::set(scheme.theme, cx);
            }
            return false;
        }

        let schemes = all_schemes(vx.query);
        let theme_index = index - 1 - stylix_offset;
        if let Some(scheme) = schemes.get(theme_index) {
            Theme::set(scheme.theme.clone(), cx);
        }
        false
    }

    fn footer_actions(&self, _vx: &ViewContext) -> Vec<(&'static str, &'static str)> {
        vec![("Apply/Fetch", "Enter"), ("Close", "Esc")]
    }
}

/// Load the Stylix system theme, if available.
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

/// Build the full scheme list from builtins + Base16 repo.
fn build_schemes() -> Vec<ThemeScheme> {
    let mut schemes = builtin_schemes();

    let repo = ThemeRepository::new(None, None);
    let base16 = repo.load_cached();
    for b16 in base16 {
        let p = &b16.palette;
        let colors = match Base16Colors::from_hex(&[
            &p.base00, &p.base01, &p.base02, &p.base03, &p.base04, &p.base05, &p.base06, &p.base07,
            &p.base08, &p.base09, &p.base0a, &p.base0b, &p.base0c, &p.base0d, &p.base0e, &p.base0f,
        ]) {
            Ok(c) => c,
            Err(_) => continue,
        };

        schemes.push(ThemeScheme {
            name: Box::leak(b16.name.into_boxed_str()),
            description: Box::leak(format!("Base16 — {}", b16.author).into_boxed_str()),
            theme: colors.to_theme(),
        });
    }

    schemes
}

/// Invalidate the cached scheme list so it reloads from disk on next access.
fn invalidate_schemes_cache() {
    let mut cache = CACHED_SCHEMES.lock().unwrap();
    *cache = None;
}

/// Get schemes filtered by query, loading from disk only on first call or after invalidation.
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

/// Compare two Hsla colors with tolerance for floating-point differences.
fn colors_match(a: gpui::Hsla, b: gpui::Hsla) -> bool {
    (a.h - b.h).abs() < 0.01 && (a.s - b.s).abs() < 0.01 && (a.l - b.l).abs() < 0.01
}

fn render_fetch_card(is_selected: bool, theme: &Theme) -> AnyElement {
    let accent_selection = theme.accent.selection;
    let interactive_hover = theme.interactive.hover;
    let text_primary = theme.text.primary;
    let text_disabled = theme.text.disabled;
    let border_default = theme.border.default;
    let accent_primary = theme.accent.primary;

    div()
        .id("theme-fetch")
        .w_full()
        .p(px(spacing::MD))
        .rounded(px(radius::LG))
        .border_1()
        .cursor_pointer()
        .when(is_selected, move |el| {
            el.bg(accent_selection).border_color(accent_primary)
        })
        .when(!is_selected, move |el| {
            el.border_color(border_default)
                .hover(move |s| s.bg(interactive_hover))
        })
        .flex()
        .items_center()
        .gap(px(spacing::SM))
        .child(
            div()
                .text_size(px(font_size::LG))
                .text_color(accent_primary)
                .child("󰇚"),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(font_size::MD))
                        .text_color(text_primary)
                        .font_weight(FontWeight::MEDIUM)
                        .child("Fetch themes from GitHub"),
                )
                .child(
                    div()
                        .text_size(px(font_size::SM))
                        .text_color(text_disabled)
                        .child("Download Base16 schemes from tinted-theming/schemes"),
                ),
        )
        .into_any_element()
}

fn render_stylix_card(
    scheme: &ThemeScheme,
    is_selected: bool,
    is_active: bool,
    theme: &Theme,
) -> AnyElement {
    let accent_selection = theme.accent.selection;
    let interactive_hover = theme.interactive.hover;
    let text_primary = theme.text.primary;
    let text_disabled = theme.text.disabled;
    let border_default = theme.border.default;
    let accent_primary = theme.accent.primary;

    let preview_colors = scheme.preview_colors();
    let name = scheme.name;
    let description = scheme.description;

    div()
        .id("theme-stylix")
        .w_full()
        .p(px(spacing::MD))
        .rounded(px(radius::LG))
        .border_1()
        .cursor_pointer()
        .when(is_selected, move |el| {
            el.bg(accent_selection).border_color(accent_primary)
        })
        .when(!is_selected, move |el| {
            el.border_color(border_default)
                .hover(move |s| s.bg(interactive_hover))
        })
        .flex()
        .flex_col()
        .gap(px(spacing::SM))
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
                        // NixOS snowflake icon
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
                                .gap(px(2.))
                                .child(
                                    div()
                                        .text_size(px(font_size::MD))
                                        .text_color(text_primary)
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(name),
                                )
                                .child(
                                    div()
                                        .text_size(px(font_size::SM))
                                        .text_color(text_disabled)
                                        .child(description),
                                ),
                        ),
                )
                .when(is_active, move |el| {
                    el.child(
                        div()
                            .px(px(spacing::SM))
                            .py(px(2.))
                            .rounded(px(radius::SM))
                            .bg(accent_primary)
                            .text_size(px(font_size::XS))
                            .text_color(text_primary)
                            .font_weight(FontWeight::BOLD)
                            .child("Active"),
                    )
                }),
        )
        .child(render_color_strip(&preview_colors))
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
    let text_primary = theme.text.primary;
    let text_disabled = theme.text.disabled;
    let border_default = theme.border.default;
    let accent_primary = theme.accent.primary;

    let preview_colors = scheme.preview_colors();
    let name = scheme.name;
    let description = scheme.description;

    div()
        .id(format!("theme-{}", name))
        .w_full()
        .p(px(spacing::MD))
        .rounded(px(radius::LG))
        .border_1()
        .cursor_pointer()
        .when(is_selected, move |el| {
            el.bg(accent_selection).border_color(accent_primary)
        })
        .when(!is_selected, move |el| {
            el.border_color(border_default)
                .hover(move |s| s.bg(interactive_hover))
        })
        .flex()
        .flex_col()
        .gap(px(spacing::SM))
        // Header row: name + active badge
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(font_size::MD))
                                .text_color(text_primary)
                                .font_weight(FontWeight::MEDIUM)
                                .child(name),
                        )
                        .child(
                            div()
                                .text_size(px(font_size::SM))
                                .text_color(text_disabled)
                                .child(description),
                        ),
                )
                .when(is_active, move |el| {
                    el.child(
                        div()
                            .px(px(spacing::SM))
                            .py(px(2.))
                            .rounded(px(radius::SM))
                            .bg(accent_primary)
                            .text_size(px(font_size::XS))
                            .text_color(text_primary)
                            .font_weight(FontWeight::BOLD)
                            .child("Active"),
                    )
                }),
        )
        // Color preview strip
        .child(render_color_strip(&preview_colors))
        .into_any_element()
}

fn render_color_strip(colors: &[gpui::Hsla]) -> AnyElement {
    let swatch_size = 20.0;

    div()
        .flex()
        .items_center()
        .gap(px(spacing::XS))
        .children(colors.iter().map(|&color| {
            div()
                .w(px(swatch_size))
                .h(px(swatch_size))
                .rounded(px(radius::SM))
                .bg(color)
        }))
        .into_any_element()
}
