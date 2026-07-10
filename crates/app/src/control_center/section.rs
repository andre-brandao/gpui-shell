//! Shared render helpers for Control Center sections (WiFi, Bluetooth).
//!
//! These capture the pieces that were duplicated verbatim between the
//! sections: header row, scrollable device list container, empty-state
//! placeholder, scan button and row action buttons.

use std::cmp::Ordering;

use gpui::{App, Div, ElementId, Hsla, MouseButton, SharedString, Stateful, div, prelude::*, px};
use ui::{ActiveTheme, icon_size, radius, spacing};

use super::tooltip::control_center_tooltip;

/// Sort items by a rank (lower first), breaking ties with a comparator.
///
/// Used to put connected devices/networks at the top of section lists.
pub(super) fn sort_ranked<T>(
    items: &mut [T],
    rank: impl Fn(&T) -> u8,
    tie: impl Fn(&T, &T) -> Ordering,
) {
    items.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| tie(a, b)));
}

/// Section header row: leading icon + title. The title takes the remaining
/// width, so any extra children (status text, scan button) end up trailing.
pub(super) fn section_header(icon: &'static str, title: &'static str, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .gap(px(spacing::SM))
        .child(
            div()
                .text_size(px(icon_size::SM))
                .text_color(theme.text.muted)
                .child(icon),
        )
        .child(
            div()
                .flex_1()
                .text_size(theme.font_sizes.sm)
                .text_color(theme.text.secondary)
                .font_weight(gpui::FontWeight::MEDIUM)
                .child(title),
        )
}

/// Scrollable, bordered container for a section's item list.
pub(super) fn section_list(id: &'static str, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    div()
        .id(id)
        .flex()
        .flex_col()
        .gap(px(2.))
        .max_h(px(240.))
        .overflow_y_scroll()
        .bg(theme.bg.primary)
        .border_1()
        .border_color(theme.border.subtle)
        .rounded(px(radius::SM))
        .py(px(spacing::XS))
}

/// Centered muted placeholder shown when a section has nothing to list.
pub(super) fn section_empty_state(message: &'static str, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .py(px(spacing::MD))
        .text_size(theme.font_sizes.sm)
        .text_color(theme.text.muted)
        .text_center()
        .child(message)
}

/// 24×24 scan/refresh button for a section header.
///
/// `active` renders the toggled-on colors (e.g. while discovering).
pub(super) fn section_scan_button(
    id: &'static str,
    active: bool,
    on_click: impl Fn(&mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let (bg, hover_bg, icon_color) = if active {
        (
            theme.interactive.toggle_on,
            theme.interactive.toggle_on_hover,
            theme.bg.primary,
        )
    } else {
        (
            theme.interactive.default,
            theme.interactive.hover,
            theme.text.muted,
        )
    };

    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .w(px(24.))
        .h(px(24.))
        .rounded(px(radius::SM))
        .cursor_pointer()
        .bg(bg)
        .hover(move |s| s.bg(hover_bg))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_click(cx))
        .child(
            div()
                .text_size(px(icon_size::SM))
                .text_color(icon_color)
                .child(super::icons::REFRESH),
        )
}

/// 22×22 icon button used for per-row actions (connect, disconnect, remove).
pub(super) fn row_action_button(
    id: String,
    icon: &'static str,
    icon_color: Hsla,
    hover_bg: Hsla,
    tooltip: &'static str,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(ElementId::Name(SharedString::from(id)))
        .w(px(22.))
        .h(px(22.))
        .rounded(px(radius::SM))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_click(cx))
        .child(
            div()
                .text_size(px(icon_size::SM))
                .text_color(icon_color)
                .child(icon),
        )
        .tooltip(control_center_tooltip(tooltip))
}
