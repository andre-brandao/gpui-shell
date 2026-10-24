//! List chrome shared by the WiFi and Bluetooth sections.

use gpui::{App, Div, ElementId, Hsla, MouseButton, SharedString, Stateful, div, prelude::*, px};
use ui::{ActiveTheme, Color, Icon, IconName, IconSize, Radius, Spacing, TextSize};

use super::tooltip::control_center_tooltip;

/// Section column with its title row; callers append trailing header items
/// through `header` and the body through the returned element.
pub fn section(
    icon: IconName,
    title: &'static str,
    header: impl FnOnce(Div) -> Div,
    cx: &App,
) -> Div {
    let header_row = div()
        .flex()
        .items_center()
        .gap(Spacing::Medium.pixels())
        .child(Icon::new(icon).size(IconSize::XSmall).color(Color::Default))
        .child(
            div()
                .flex_1()
                .text_size(TextSize::Small.rems())
                .text_color(cx.theme().colors.text)
                .font_weight(gpui::FontWeight::MEDIUM)
                .child(title),
        );

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(Spacing::Medium.pixels())
        .child(header(header_row))
}

/// Small muted line under the header, e.g. "Scanning...".
pub fn hint(text: &'static str, cx: &App) -> Div {
    div()
        .text_size(TextSize::XSmall.rems())
        .text_color(cx.theme().colors.text_muted)
        .child(text)
}

/// Centered placeholder for an empty or unavailable list.
pub fn empty_state(text: &'static str, cx: &App) -> Div {
    div()
        .py(Spacing::Large.pixels())
        .text_size(TextSize::Small.rems())
        .text_color(cx.theme().colors.text_muted)
        .text_center()
        .child(text)
}

/// Scrollable framed list holding the rows.
pub fn list(id: &'static str, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    div()
        .id(id)
        .flex()
        .flex_col()
        .gap(px(2.))
        .max_h(px(240.))
        .overflow_y_scroll()
        .bg(theme.colors.background)
        .border_1()
        .border_color(theme.colors.border_variant)
        .rounded(Radius::Small.pixels())
        .py(Spacing::XSmall.pixels())
}

/// Clickable row, highlighted while `selected` (the connected entry).
pub fn row(id: impl Into<SharedString>, selected: bool, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let selected_bg = theme.colors.element_selected;
    let hover_bg = theme.colors.element_hover;
    div()
        .id(ElementId::Name(id.into()))
        .flex()
        .items_center()
        .gap(Spacing::Medium.pixels())
        .w_full()
        .px(Spacing::Medium.pixels())
        .py(Spacing::XSmall.pixels())
        .rounded(Radius::Small.pixels())
        .cursor_pointer()
        .when(selected, |el| el.bg(selected_bg))
        .when(!selected, |el| el.hover(move |s| s.bg(hover_bg)))
}

/// Row label that takes the remaining width.
pub fn row_label(text: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .flex_1()
        .text_size(TextSize::Small.rems())
        .text_color(cx.theme().colors.text)
        .overflow_hidden()
        .child(text.into())
}

/// Leading icon tinted with the accent while `active`.
pub fn row_icon(
    id: impl Into<SharedString>,
    icon: IconName,
    active: bool,
    tooltip: impl Into<SharedString>,
    cx: &App,
) -> Stateful<Div> {
    let theme = cx.theme();
    let color = if active {
        theme.colors.accent
    } else {
        theme.colors.text_muted
    };
    badge(id, icon, color, tooltip)
}

/// Non-interactive icon with a tooltip.
pub fn badge(
    id: impl Into<SharedString>,
    icon: IconName,
    color: Hsla,
    tooltip: impl Into<SharedString>,
) -> Stateful<Div> {
    div()
        .id(ElementId::Name(id.into()))
        .child(
            Icon::new(icon)
                .size(IconSize::XSmall)
                .color(Color::Custom(color)),
        )
        .tooltip(control_center_tooltip(tooltip.into()))
}

/// Trailing icon button on a row.
pub fn action_button(
    id: impl Into<SharedString>,
    icon: IconName,
    color: Hsla,
    tooltip: &'static str,
    on_click: impl Fn(&mut App) + 'static,
    cx: &App,
) -> Stateful<Div> {
    let hover_bg = cx.theme().colors.element_hover;
    div()
        .id(ElementId::Name(id.into()))
        .w(px(22.))
        .h(px(22.))
        .rounded(Radius::Small.pixels())
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg))
        .tooltip(control_center_tooltip(tooltip))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            // The row under it has its own click.
            cx.stop_propagation();
            on_click(cx)
        })
        .child(
            Icon::new(icon)
                .size(IconSize::XSmall)
                .color(Color::Custom(color)),
        )
}

/// Square button for the header, e.g. rescan.
pub fn header_button(id: &'static str, cx: &App) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .w(px(24.))
        .h(px(24.))
        .rounded(Radius::Small.pixels())
        .bg(cx.theme().colors.element_background)
}
