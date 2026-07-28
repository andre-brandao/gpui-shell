//! [`ElevationIndex`] - semantic z-axis levels for shadowed surfaces.
//!
//! Engram leans on a tiny vocabulary of "how high above the page does
//! this surface float?" so that any given component (popover, modal,
//! notification) can ask for the right shadow stack without inventing
//! its own. The values here are eyeballed against zed's own
//! `ui::ElevationIndex`, minus zed-specific levels we don't need
//! (`EditorSurface`).

use std::fmt::{self, Display, Formatter};

use crate::theme::{ActiveTheme, Appearance};
use gpui::{App, BoxShadow, hsla, point, px};

/// Semantic z-axis level for a surface.
///
/// Higher variants float higher above the app background and therefore
/// cast a stronger shadow. Use [`ElevationIndex::shadow`] to obtain the
/// concrete `BoxShadow` stack to apply via `.shadow(...)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElevationIndex {
    /// The bottom layer - flush with the app background. No shadow.
    Background,
    /// The standard surface layer used for panels, cards, and inline
    /// containers. No shadow; relies on borders/background contrast.
    Surface,
    /// One step above [`Self::Surface`] - used for floating-but-not-modal
    /// chrome like popovers, dropdown menus, and notifications.
    ElevatedSurface,
    /// The topmost layer for modals/dialogs that block the rest of the UI.
    ModalSurface,
}

impl Display for ElevationIndex {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ElevationIndex::Background => write!(f, "Background"),
            ElevationIndex::Surface => write!(f, "Surface"),
            ElevationIndex::ElevatedSurface => write!(f, "Elevated Surface"),
            ElevationIndex::ModalSurface => write!(f, "Modal Surface"),
        }
    }
}

impl ElevationIndex {
    /// The box-shadow stack appropriate for this elevation, given the
    /// active theme's [`Appearance`]. Returns an empty vector for levels
    /// that should not cast a shadow.
    pub fn shadow(self, cx: &App) -> Vec<BoxShadow> {
        let is_light = cx.theme().appearance() == Appearance::Light;

        match self {
            ElevationIndex::Background | ElevationIndex::Surface => vec![],

            ElevationIndex::ElevatedSurface => vec![
                BoxShadow {
                    color: hsla(0., 0., 0., 0.12),
                    offset: point(px(0.), px(2.)),
                    blur_radius: px(3.),
                    spread_radius: px(0.),
                    inset: false,
                },
                BoxShadow {
                    color: hsla(0., 0., 0., if is_light { 0.03 } else { 0.06 }),
                    offset: point(px(0.), px(1.)),
                    blur_radius: px(0.),
                    spread_radius: px(0.),
                    inset: false,
                },
            ],

            ElevationIndex::ModalSurface => vec![
                BoxShadow {
                    color: hsla(0., 0., 0., if is_light { 0.06 } else { 0.12 }),
                    offset: point(px(0.), px(2.)),
                    blur_radius: px(3.),
                    spread_radius: px(0.),
                    inset: false,
                },
                BoxShadow {
                    color: hsla(0., 0., 0., if is_light { 0.06 } else { 0.08 }),
                    offset: point(px(0.), px(3.)),
                    blur_radius: px(6.),
                    spread_radius: px(0.),
                    inset: false,
                },
                BoxShadow {
                    color: hsla(0., 0., 0., 0.04),
                    offset: point(px(0.), px(6.)),
                    blur_radius: px(12.),
                    spread_radius: px(0.),
                    inset: false,
                },
                BoxShadow {
                    color: hsla(0., 0., 0., if is_light { 0.04 } else { 0.12 }),
                    offset: point(px(0.), px(1.)),
                    blur_radius: px(0.),
                    spread_radius: px(0.),
                    inset: false,
                },
            ],
        }
    }
}
