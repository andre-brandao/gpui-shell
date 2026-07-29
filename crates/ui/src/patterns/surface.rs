//! Chrome for the shell's floating surfaces.
//!
//! A `Styled` extension rather than a component, because every one of these
//! surfaces is a `div()` that also owns an id, a focus handle, a scroll
//! handle or a click target. Wrapping them in an element would push all of
//! that a level down; a trait method just sets the paint.

use gpui::{App, Styled};

use crate::{ActiveTheme, Radius};

/// The two surface levels the shell paints with.
pub trait PanelSurface: Styled + Sized {
    /// A floating panel: the control center, a bar popup, a notification.
    /// Opaque background, hairline border, large radius.
    fn panel_surface(self, cx: &App) -> Self {
        let colors = cx.theme().colors();
        self.bg(colors.background)
            .border_1()
            .border_color(colors.border)
            .rounded(Radius::Large.pixels())
    }

    /// A card *inside* a panel - one slider, one expanded section. Lifted
    /// off the panel background, quieter border, tighter radius.
    fn panel_card(self, cx: &App) -> Self {
        let colors = cx.theme().colors();
        self.bg(colors.surface_background)
            .border_1()
            .border_color(colors.border_variant)
            .rounded(Radius::Medium.pixels())
    }
}

impl<T: Styled> PanelSurface for T {}
