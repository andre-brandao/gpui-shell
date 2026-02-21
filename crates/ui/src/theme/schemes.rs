//! Built-in theme schemes.
//!
//! Each scheme provides a complete `Theme` with all color groups defined.
//! Themes are also loaded from Stylix and downloaded from GitHub.

use gpui::Hsla;

use super::Theme;

/// A named theme scheme.
#[derive(Clone)]
pub struct ThemeScheme {
    /// Display name.
    pub name: &'static str,
    /// Short description.
    pub description: &'static str,
    /// The full theme.
    pub theme: Theme,
}

impl ThemeScheme {
    /// Extract a representative set of colors for preview swatches.
    pub fn preview_colors(&self) -> Vec<Hsla> {
        let t = &self.theme;
        vec![
            t.bg.primary,
            t.bg.secondary,
            t.bg.elevated,
            t.accent.primary,
            t.accent.hover,
            t.status.success,
            t.status.warning,
            t.status.error,
            t.text.primary,
            t.text.muted,
        ]
    }
}

/// Return all built-in theme schemes.
pub fn builtin_schemes() -> Vec<ThemeScheme> {
    vec![default_scheme()]
}

fn default_scheme() -> ThemeScheme {
    ThemeScheme {
        name: "Default",
        description: "Dark theme with blue accents",
        theme: Theme::default(),
    }
}
