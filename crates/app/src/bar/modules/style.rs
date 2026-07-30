//! Content helpers for bar widgets: sizes, gaps, and the colour decisions
//! the bar config drives.
//!
//! The chip a widget sits in, and the bar surface under it, are
//! [`ui::patterns::BarChip`] and [`ui::patterns::BarSurface`] - this module
//! only decides what to hand them.
//!
//! Font sizes come from `TextSize` (XSmall/Small for vertical, Small/Medium for horizontal).

use gpui::{AnyElement, Hsla, IntoElement, SharedString, prelude::*, px};
use ui::{
    Color, Divider, Icon, IconName, IconSize, Label, LabelCommon, Spacing, TextSize, Theme, h_flex,
    v_flex,
};

use crate::bar::config::BarConfig;

/// Common gap used inside compact bar widgets.
pub const CHIP_GAP: f32 = Spacing::XSmall.value();
/// Tighter gap used inside grouped widgets like workspaces and tray.
pub const GROUP_GAP: f32 = Spacing::XSmall.value();
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
/// Fixed text line width for vertically stacked labels so each line shares one center axis.
pub const VERTICAL_TEXT_LINE_WIDTH: f32 = 20.0;

#[inline(always)]
pub fn group_gap(is_vertical: bool) -> f32 {
    if is_vertical { GROUP_GAP } else { 3.0 }
}

/// Icon size tuned for bar density.
#[inline(always)]
pub fn icon(is_vertical: bool) -> IconSize {
    if is_vertical {
        IconSize::Small
    } else {
        IconSize::Medium
    }
}

/// The label size for the given bar orientation.
///
/// Returns a [`TextSize`] rather than absolute pixels so the label still
/// scales with the user's configured base font size.
#[inline(always)]
pub fn label_size(is_vertical: bool) -> TextSize {
    if is_vertical {
        TextSize::XSmall
    } else {
        TextSize::Small
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
pub fn widget_background(theme: &Theme, bar: &BarConfig) -> Hsla {
    if bar.widget_background {
        theme.colors.surface_background
    } else {
        theme.colors.border_transparent
    }
}

#[inline(always)]
pub fn widget_border(theme: &Theme, bar: &BarConfig, is_vertical: bool) -> Hsla {
    if bar.widget_border {
        if is_vertical {
            theme.colors.border_variant.opacity(0.9)
        } else {
            theme.colors.border
        }
    } else {
        theme.colors.border_transparent
    }
}

/// Resting fill of an interactive widget under the pointer.
#[inline(always)]
pub fn widget_hover_background(theme: &Theme, bar: &BarConfig) -> Hsla {
    if bar.widget_background {
        theme.colors.elevated_surface_background
    } else {
        theme.colors.element_hover
    }
}

/// The label a bar widget puts next to its icon, in the size and color the
/// orientation and state ask for.
pub fn bar_label(text: impl Into<SharedString>, is_vertical: bool, color: Hsla) -> Label {
    Label::new(text)
        .size(label_size(is_vertical))
        .color(Color::Custom(color))
}

/// Render a compact icon/value pair for status widgets.
pub fn bar_stat(
    is_vertical: bool,
    icon_name: IconName,
    value_text: impl Into<SharedString>,
    color: Hsla,
) -> AnyElement {
    let label = bar_label(value_text, is_vertical, color);

    stack(is_vertical)
        .justify_center()
        .gap(px(CHIP_GAP))
        .child(
            Icon::new(icon_name)
                .size(icon(is_vertical))
                .color(Color::Custom(color)),
        )
        .child(if is_vertical {
            vertical_text_line(label)
        } else {
            label.into_any_element()
        })
        .into_any_element()
}

/// The flex container a widget lays its content out in: a column on a
/// vertical bar, a row on a horizontal one, centered either way.
pub fn stack(is_vertical: bool) -> gpui::Div {
    if is_vertical {
        v_flex().items_center()
    } else {
        h_flex()
    }
}

/// The subtle divider used inside dense horizontal widgets. Takes its colour
/// from the widget border, which the bar config can switch off entirely.
pub fn section_divider(color: Hsla) -> Divider {
    Divider::vertical()
        .color(color)
        .length(px(SECTION_DIVIDER_HEIGHT))
}

/// Center a vertical text row to a fixed axis so stacked labels do not drift visually.
pub fn vertical_text_line(content: impl IntoElement) -> AnyElement {
    h_flex()
        .w(px(VERTICAL_TEXT_LINE_WIDTH))
        .justify_center()
        .child(content)
        .into_any_element()
}
