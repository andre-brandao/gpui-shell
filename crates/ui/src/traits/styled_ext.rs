//! [`StyledExt`] - layout and elevation shorthands on top of [`gpui::Styled`].

use crate::theme::ActiveTheme;
use gpui::{App, Styled};

use crate::styles::ElevationIndex;

fn elevated<E: Styled>(this: E, cx: &App, index: ElevationIndex) -> E {
    let colors = cx.theme().colors();
    this.bg(colors.elevated_surface_background)
        .rounded_lg()
        .border_1()
        .border_color(colors.border_variant)
        .shadow(index.shadow(cx))
}

/// Shorthand methods on top of [`gpui::Styled`].
pub trait StyledExt: Styled + Sized {
    /// Horizontal flex row with centered children.
    fn h_flex(self) -> Self {
        self.flex().flex_row().items_center()
    }

    /// Vertical flex column.
    fn v_flex(self) -> Self {
        self.flex().flex_col()
    }

    /// In-page card: background, rounded corners, border, no shadow.
    fn elevation_1(self, cx: &App) -> Self {
        elevated(self, cx, ElevationIndex::Surface)
    }

    /// Popover / dropdown / toast surface: soft drop shadow.
    fn elevation_2(self, cx: &App) -> Self {
        elevated(self, cx, ElevationIndex::ElevatedSurface)
    }

    /// Modal / dialog surface: deepest shadow stack.
    fn elevation_3(self, cx: &App) -> Self {
        elevated(self, cx, ElevationIndex::ModalSurface)
    }

    /// Theme border color.
    fn border_primary(self, cx: &App) -> Self {
        self.border_color(cx.theme().colors().border)
    }

    /// Theme muted border color.
    fn border_muted(self, cx: &App) -> Self {
        self.border_color(cx.theme().colors().border_variant)
    }

    /// Theme keyboard-focus border color.
    fn border_focused(self, cx: &App) -> Self {
        self.border_color(cx.theme().colors().border_focused)
    }
}

impl<E: Styled + Sized> StyledExt for E {}
