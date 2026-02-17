//! Shared sizing and spacing helpers for bar widgets.

use ui::{font_size, icon_size, spacing};

/// Common gap used inside compact bar widgets.
pub const CHIP_GAP: f32 = spacing::XS;
/// Common vertical padding used inside compact bar widgets.
pub const CHIP_PADDING_Y: f32 = 3.0;
/// Standard tray icon button size.
pub const TRAY_ITEM_SIZE: f32 = 24.0;
/// Workspace pill height.
pub const WORKSPACE_PILL_HEIGHT: f32 = 20.0;
/// Workspace pill width (inactive).
pub const WORKSPACE_PILL_WIDTH: f32 = 20.0;
/// Workspace pill width (active).
pub const WORKSPACE_PILL_WIDTH_ACTIVE: f32 = 24.0;
/// Horizontal workspace pill width (inactive).
pub const WORKSPACE_PILL_WIDTH_HORIZONTAL: f32 = 24.0;
/// Horizontal workspace pill width (active).
pub const WORKSPACE_PILL_WIDTH_HORIZONTAL_ACTIVE: f32 = 30.0;
/// Horizontal section divider height.
pub const SECTION_DIVIDER_HEIGHT: f32 = 14.0;

/// Horizontal padding for compact bar widgets.
#[inline(always)]
pub fn chip_padding_x(is_vertical: bool) -> f32 {
    if is_vertical {
        spacing::XS
    } else {
        spacing::SM
    }
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

/// Label text size tuned for bar density.
#[inline(always)]
pub fn label(is_vertical: bool) -> f32 {
    if is_vertical {
        font_size::XS
    } else {
        font_size::SM
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
