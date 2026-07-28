//! [`StyledExt`] - engram-flavoured shorthands on top of [`gpui::Styled`].
//!
//! These methods are pure compositions of the underlying GPUI fluent API.
//! They exist so layout and elevation idioms ("a horizontal stack of
//! centered children", "a card that floats one level above the page")
//! show up the same way at every call site, instead of being open-coded
//! into half a dozen subtly different forms.
//!
//! The trait is implemented for **every** `E: Styled` via a blanket impl,
//! so any element produced by GPUI's fluent API picks the methods up for
//! free - no opt-in required.

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

/// Engram-specific shorthand methods on top of [`gpui::Styled`].
///
/// Mirrors the most-used helpers from zed's `ui::StyledExt`, scoped down
/// to the bits engram actually exercises today.
pub trait StyledExt: Styled + Sized {
    /// Horizontal flex row with centered children.
    ///
    /// Sets `flex()`, `flex_row()`, `items_center()`.
    fn h_flex(self) -> Self {
        self.flex().flex_row().items_center()
    }

    /// Vertical flex column.
    ///
    /// Sets `flex()`, `flex_col()`.
    fn v_flex(self) -> Self {
        self.flex().flex_col()
    }

    /// Render `self` as a [`Surface`](ElevationIndex::Surface) - the standard
    /// in-page card. Sets background, rounded corners, border, and the
    /// elevation's shadow stack (currently empty for `Surface`).
    fn elevation_1(self, cx: &App) -> Self {
        elevated(self, cx, ElevationIndex::Surface)
    }

    /// Render `self` as an [`ElevatedSurface`](ElevationIndex::ElevatedSurface)
    /// - popovers, dropdown menus, toasts. Adds a soft drop shadow.
    fn elevation_2(self, cx: &App) -> Self {
        elevated(self, cx, ElevationIndex::ElevatedSurface)
    }

    /// Render `self` as a [`ModalSurface`](ElevationIndex::ModalSurface) -
    /// modals and dialogs. Adds the deepest shadow stack.
    fn elevation_3(self, cx: &App) -> Self {
        elevated(self, cx, ElevationIndex::ModalSurface)
    }

    /// Apply the theme's primary border color.
    fn border_primary(self, cx: &App) -> Self {
        self.border_color(cx.theme().colors().border)
    }

    /// Apply the theme's muted (subtle) border color.
    fn border_muted(self, cx: &App) -> Self {
        self.border_color(cx.theme().colors().border_variant)
    }

    /// Apply the theme's keyboard-focus border color. Used by components
    /// that grow a focus ring on `:focus` (text inputs, buttons on tab
    /// navigation, etc).
    fn border_focused(self, cx: &App) -> Self {
        self.border_color(cx.theme().colors().border_focused)
    }
}

impl<E: Styled + Sized> StyledExt for E {}
