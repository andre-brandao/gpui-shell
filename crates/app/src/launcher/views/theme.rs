//! Theme browser view for the launcher.
//!
//! Lists available theme schemes with color preview swatches.
//! Selecting a theme applies it globally.

use crate::launcher::view::{LauncherView, ViewContext};
use gpui::{AnyElement, App, FontWeight, div, prelude::*, px};
use ui::{ActiveTheme, Theme, ThemeScheme, builtin_schemes, font_size, radius, spacing};

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
        let schemes = filtered_schemes(vx.query);
        let count = schemes.len();

        let current_accent = theme.accent.primary;

        let element = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(spacing::SM))
            .p(px(spacing::SM))
            .children(schemes.into_iter().enumerate().map(|(i, scheme)| {
                let is_selected = i == vx.selected_index;
                let is_active = colors_match(scheme.theme.accent.primary, current_accent)
                    && colors_match(scheme.theme.bg.primary, theme.bg.primary);
                render_theme_card(&scheme, is_selected, is_active, theme)
            }))
            .into_any_element();

        (element, count)
    }

    fn on_select(&self, index: usize, vx: &ViewContext, cx: &mut App) -> bool {
        let schemes = filtered_schemes(vx.query);
        if let Some(scheme) = schemes.get(index) {
            Theme::set(scheme.theme.clone(), cx);
        }
        false
    }

    fn footer_actions(&self, _vx: &ViewContext) -> Vec<(&'static str, &'static str)> {
        vec![("Apply", "Enter"), ("Close", "Esc")]
    }
}

fn filtered_schemes(query: &str) -> Vec<ThemeScheme> {
    let query_lower = query.to_lowercase();
    builtin_schemes()
        .into_iter()
        .filter(|s| {
            query.is_empty()
                || s.name.to_lowercase().contains(&query_lower)
                || s.description.to_lowercase().contains(&query_lower)
        })
        .collect()
}

/// Compare two Hsla colors with tolerance for floating-point differences.
fn colors_match(a: gpui::Hsla, b: gpui::Hsla) -> bool {
    (a.h - b.h).abs() < 0.01 && (a.s - b.s).abs() < 0.01 && (a.l - b.l).abs() < 0.01
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
