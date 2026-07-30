//! Chrome for the shell's floating surfaces.

use gpui::{App, Styled};

use crate::{ActiveTheme, Radius};

/// The two surface levels the shell paints with.
pub trait PanelSurface: Styled + Sized {
    /// A floating panel: the control center, a bar popup, a notification.
    fn panel_surface(self, cx: &App) -> Self {
        let colors = cx.theme().colors();
        self.bg(colors.background)
            .border_1()
            .border_color(colors.border)
            .rounded(Radius::Large.pixels())
    }

    /// A card *inside* a panel - one slider, one expanded section.
    fn panel_card(self, cx: &App) -> Self {
        let colors = cx.theme().colors();
        self.bg(colors.surface_background)
            .border_1()
            .border_color(colors.border_variant)
            .rounded(Radius::Medium.pixels())
    }
}

impl<T: Styled> PanelSurface for T {}
