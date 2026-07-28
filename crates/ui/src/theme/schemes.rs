//! A named, selectable Base16 palette.
//!
//! Schemes are what the theme launcher lists: a palette plus enough
//! metadata to render a card for it. They carry the *palette*, not a
//! resolved [`Theme`](super::Theme), so the user's font size and any token
//! overrides stay independent of which scheme is picked.

use gpui::{Hsla, SharedString};

use super::base16::Base16Palette;

/// A named Base16 scheme - built in, from Stylix, or fetched from a
/// Tinted Theming repository.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeScheme {
    /// Display name.
    pub name: SharedString,
    /// Short description (typically `"<provider> — <author>"`).
    pub description: SharedString,
    /// The 16 colors this scheme is made of.
    pub palette: Base16Palette,
}

impl ThemeScheme {
    pub fn new(
        name: impl Into<SharedString>,
        description: impl Into<SharedString>,
        palette: Base16Palette,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            palette,
        }
    }

    /// Colors for the preview strip on a scheme card: the surface ramp
    /// followed by the eight accents.
    pub fn preview_colors(&self) -> Vec<Hsla> {
        let p = &self.palette;
        vec![
            p.base00, p.base01, p.base02, p.base05, p.base08, p.base09, p.base0a, p.base0b,
            p.base0c, p.base0d, p.base0e, p.base0f,
        ]
    }
}

/// The schemes that ship with the shell, available before any theme
/// repository has been cloned.
pub fn builtin_schemes() -> Vec<ThemeScheme> {
    vec![ThemeScheme::new(
        "Default Dark",
        "Built-in — the canonical Base16 dark scheme",
        Base16Palette::default(),
    )]
}
