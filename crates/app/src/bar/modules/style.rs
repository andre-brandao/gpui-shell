//! Shared sizing and spacing helpers for bar widgets.
//!
//! Font sizes should be accessed from `theme.font_sizes` (xs/sm for vertical, sm/md for horizontal).

use gpui::{AnyElement, Hsla, IntoElement, div, prelude::*, px};
use ui::{Theme, icon_size, radius, spacing};

use crate::bar::config::BarConfig;

/// Common gap used inside compact bar widgets.
pub const CHIP_GAP: f32 = spacing::XS;
/// Tighter gap used inside grouped widgets like workspaces and tray.
pub const GROUP_GAP: f32 = spacing::XS;
/// Common vertical padding used inside compact bar widgets.
pub const CHIP_PADDING_Y: f32 = 3.0;
/// Shared outer breathing room around each widget shell.
pub const CHIP_OUTER_MARGIN: f32 = 2.0;
/// Section gap between widgets once per-widget outer breathing room is applied.
pub const BAR_SECTION_GAP: f32 = 0.0;
/// Standard tray icon button size.
pub const TRAY_ITEM_SIZE: f32 = 24.0;
/// Workspace pill height.
pub const WORKSPACE_PILL_HEIGHT: f32 = 20.0;
/// Workspace pill width (inactive).
pub const WORKSPACE_PILL_WIDTH: f32 = 20.0;
/// Workspace pill width (active).
pub const WORKSPACE_PILL_WIDTH_ACTIVE: f32 = 24.0;
/// Horizontal workspace pill width (inactive).
pub const WORKSPACE_PILL_WIDTH_HORIZONTAL: f32 = 22.0;
/// Horizontal workspace pill width (active).
pub const WORKSPACE_PILL_WIDTH_HORIZONTAL_ACTIVE: f32 = 28.0;
/// Horizontal section divider height.
pub const SECTION_DIVIDER_HEIGHT: f32 = 14.0;

#[inline(always)]
fn shell_radius(is_vertical: bool) -> f32 {
    if is_vertical { radius::SM } else { radius::LG }
}

#[inline(always)]
fn shell_padding_y(is_vertical: bool) -> f32 {
    if is_vertical {
        CHIP_PADDING_Y
    } else {
        CHIP_PADDING_Y + 1.0
    }
}

#[inline(always)]
fn shell_height(is_vertical: bool) -> Option<f32> {
    if is_vertical { None } else { Some(24.0) }
}

/// Horizontal padding for compact bar widgets.
#[inline(always)]
pub fn chip_padding_x(is_vertical: bool) -> f32 {
    if is_vertical {
        spacing::XS + 1.0
    } else {
        spacing::SM
    }
}

#[inline(always)]
pub fn group_gap(is_vertical: bool) -> f32 {
    if is_vertical { GROUP_GAP } else { 3.0 }
}

#[inline(always)]
fn group_padding_x(is_vertical: bool) -> f32 {
    if is_vertical { 2.0 } else { spacing::SM - 1.0 }
}

#[inline(always)]
fn widget_outer_margin_x(is_vertical: bool) -> f32 {
    if is_vertical { CHIP_OUTER_MARGIN } else { 2.0 }
}

/// Icon size tuned for bar density.
#[inline(always)]
pub fn icon(is_vertical: bool) -> f32 {
    if is_vertical {
        icon_size::MD
    } else {
        icon_size::LG
    }
}

/// Get the appropriate label font size from theme based on bar orientation.
///
/// Use `label_size(theme, is_vertical)` instead of the old `style::label()`.
#[inline(always)]
pub fn label_size(theme: &ui::Theme, is_vertical: bool) -> gpui::Pixels {
    if is_vertical {
        theme.font_sizes.xs
    } else {
        theme.font_sizes.sm
    }
}

/// Format a percentage compactly in vertical mode.
#[inline(always)]
pub fn compact_percent(value: u32, is_vertical: bool) -> String {
    if is_vertical {
        value.to_string()
    } else {
        format!("{value}%")
    }
}

#[inline(always)]
pub fn widget_background(theme: &Theme, bar: &BarConfig, is_vertical: bool) -> Hsla {
    if bar.widget_background {
        if is_vertical {
            theme.bg.secondary
        } else {
            theme.bg.secondary
        }
    } else {
        theme.transparent
    }
}

#[inline(always)]
pub fn group_background(theme: &Theme, bar: &BarConfig, is_vertical: bool) -> Hsla {
    if bar.widget_background {
        if is_vertical {
            theme.bg.secondary
        } else {
            theme.bg.secondary
        }
    } else {
        theme.transparent
    }
}

#[inline(always)]
pub fn widget_border(theme: &Theme, bar: &BarConfig, is_vertical: bool) -> Hsla {
    if bar.widget_border {
        if is_vertical {
            theme.border.subtle.opacity(0.9)
        } else {
            theme.border.default
        }
    } else {
        theme.transparent
    }
}

/// Apply the compact shared shell used by single bar widgets.
pub fn bar_widget_shell(
    theme: &Theme,
    bar: &BarConfig,
    is_vertical: bool,
    is_interactive: bool,
    content: impl IntoElement,
) -> AnyElement {
    let hover_bg = if bar.widget_background {
        if is_vertical {
            theme.bg.tertiary
        } else {
            theme.bg.tertiary
        }
    } else {
        theme.interactive.hover
    };

    div()
        .when(is_vertical, |el| el.flex().flex_col().items_center())
        .when(!is_vertical, |el| el.flex().items_center())
        .px(px(chip_padding_x(is_vertical)))
        .py(px(shell_padding_y(is_vertical)))
        .when_some(shell_height(is_vertical), |el, height| el.h(px(height)))
        .rounded(px(shell_radius(is_vertical)))
        .bg(widget_background(theme, bar, is_vertical))
        .when(bar.widget_border, |el| {
            el.border_1()
                .border_color(widget_border(theme, bar, is_vertical))
        })
        .when(is_interactive, |el| {
            el.cursor_pointer().hover(move |style| style.bg(hover_bg))
        })
        .child(content)
        .into_any_element()
}

/// Apply the shared outer spacing used by all visible bar widgets.
pub fn bar_widget_slot(is_vertical: bool, content: impl IntoElement) -> AnyElement {
    if is_vertical {
        div()
            .w_full()
            .flex()
            .items_center()
            .justify_center()
            .py(px(CHIP_OUTER_MARGIN))
            .child(content)
            .into_any_element()
    } else {
        div()
            .mx(px(widget_outer_margin_x(is_vertical)))
            .my(px(CHIP_OUTER_MARGIN))
            .child(content)
            .into_any_element()
    }
}

/// Apply the grouped shell used by composite bar widgets.
pub fn bar_group_shell(
    theme: &Theme,
    bar: &BarConfig,
    is_vertical: bool,
    is_interactive: bool,
    content: impl IntoElement,
) -> AnyElement {
    let hover_bg = if bar.widget_background {
        if is_vertical {
            theme.bg.tertiary
        } else {
            theme.bg.tertiary
        }
    } else {
        theme.interactive.hover
    };

    div()
        .when(is_vertical, |el| el.flex().flex_col().items_center())
        .when(!is_vertical, |el| el.flex().items_center())
        .px(px(group_padding_x(is_vertical)))
        .py(px(shell_padding_y(is_vertical)))
        .when_some(shell_height(is_vertical), |el, height| el.h(px(height)))
        .rounded(px(shell_radius(is_vertical)))
        .bg(group_background(theme, bar, is_vertical))
        .when(bar.widget_border, |el| {
            el.border_1()
                .border_color(widget_border(theme, bar, is_vertical))
        })
        .when(is_interactive, |el| {
            el.cursor_pointer().hover(move |style| style.bg(hover_bg))
        })
        .child(content)
        .into_any_element()
}

/// Render a compact icon/value pair for status widgets.
pub fn bar_stat(
    theme: &Theme,
    is_vertical: bool,
    icon_text: &'static str,
    value_text: impl IntoElement,
    color: Hsla,
) -> AnyElement {
    div()
        .flex()
        .when(is_vertical, |el| el.flex_col())
        .items_center()
        .gap(px(CHIP_GAP))
        .child(
            div()
                .text_size(px(icon(is_vertical)))
                .text_color(color)
                .child(icon_text),
        )
        .child(
            div()
                .text_size(label_size(theme, is_vertical))
                .text_color(color)
                .child(value_text),
        )
        .into_any_element()
}

/// Render the subtle divider used inside dense horizontal widgets.
pub fn section_divider(color: Hsla) -> AnyElement {
    div()
        .w(px(1.0))
        .h(px(SECTION_DIVIDER_HEIGHT))
        .bg(color)
        .into_any_element()
}
