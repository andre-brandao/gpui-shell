//! The theme system.
//!
//! # Layering
//!
//! ```text
//! Base16 palette (16 colors)     input: Tinted Theming scheme, Stylix,
//!         |                             matugen(wallpaper), or hand-written
//!         |  Base16Palette::into_colors()
//!         v
//! ThemeColors (~50 tokens)       what components read
//!         |
//!         |  ThemeColorsRefinement::refine()
//!         v
//! Theme                          resolved, plus the user's font size
//! ```
//!
//! The two ends are deliberately separate. A palette is a *source of
//! colors*; the token set is a *set of roles*. Keeping them apart is what
//! lets the wallpaper regenerate the palette without discarding the user's
//! hand-picked accent, and what lets components speak in roles
//! (`element_hover`) instead of palette slots (`base02`).
//!
//! # Usage
//!
//! ```ignore
//! use ui::{ActiveTheme, TextSize};
//!
//! fn render(&mut self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
//!     let colors = cx.theme().colors();
//!     div()
//!         .bg(colors.surface_background)
//!         .text_color(colors.text)
//!         .border_color(colors.border)
//!         .text_size(TextSize::Small.rems())
//! }
//! ```

use gpui::{App, Global, Pixels, SharedString, px};
use serde::{Deserialize, Serialize};

mod base16;
mod color_string;
mod colors;
#[macro_use]
mod refineable;
mod refinement;
mod schemes;
mod tokens;

pub use base16::Base16Palette;
pub use colors::{Color, StatusColors, ThemeColors};
pub use refinement::{StatusColorsRefinement, ThemeColorsRefinement};
pub use schemes::{ThemeScheme, builtin_schemes};
pub use tokens::{IconSize, Radius, Spacing, TextSize};

/// Whether a theme is a light or a dark variant.
///
/// Derived from the palette's background lightness - components branch on
/// it for the handful of decisions that genuinely differ (shadow strength,
/// overlay direction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    Light,
    Dark,
}

impl Appearance {
    pub fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }
}

/// The active theme, stored as a gpui [`Global`].
///
/// Holds both the resolved [`ThemeColors`] components read *and* the
/// palette plus overrides they were derived from, so the theme can be
/// written back to disk without losing the distinction between "the scheme
/// I picked" and "the token I overrode".
#[derive(Debug, Clone)]
pub struct Theme {
    /// Display name of the underlying scheme.
    pub name: SharedString,
    /// Light or dark, derived from the palette.
    pub appearance: Appearance,
    /// Resolved semantic tokens: palette expansion with `overrides` applied.
    pub colors: ThemeColors,
    /// The palette `colors` was derived from.
    pub palette: Base16Palette,
    /// User overrides layered on top of the palette expansion.
    pub overrides: ThemeColorsRefinement,
    /// Base font size. Installed as the window's rem size, so every
    /// [`TextSize`] scales with it.
    pub font_size: Pixels,
}

impl Global for Theme {}

impl Theme {
    /// Default base font size, matching what the shell shipped before
    /// sizes became configurable tokens.
    pub const DEFAULT_FONT_SIZE: Pixels = px(13.0);

    /// Build a theme from a palette, with no overrides.
    pub fn from_palette(name: impl Into<SharedString>, palette: Base16Palette) -> Self {
        Self {
            name: name.into(),
            appearance: palette.appearance(),
            colors: palette.into_colors(),
            palette,
            overrides: ThemeColorsRefinement::default(),
            font_size: Self::DEFAULT_FONT_SIZE,
        }
    }

    /// The resolved semantic tokens.
    #[inline(always)]
    pub fn colors(&self) -> &ThemeColors {
        &self.colors
    }

    /// Swap the palette, keeping the current overrides and font size.
    ///
    /// This is the "user picked a different scheme" path, and the
    /// "wallpaper changed" path.
    pub fn set_palette(&mut self, name: impl Into<SharedString>, palette: Base16Palette) {
        self.name = name.into();
        self.appearance = palette.appearance();
        self.palette = palette;
        self.resolve();
    }

    /// Replace the user overrides, keeping the current palette.
    pub fn set_overrides(&mut self, overrides: ThemeColorsRefinement) {
        self.overrides = overrides;
        self.resolve();
    }

    /// Recompute [`colors`](Self::colors) from the palette and overrides.
    fn resolve(&mut self) {
        self.colors = self.palette.into_colors();
        self.overrides.refine(&mut self.colors);
    }

    /// Initialize the global theme. Call once at startup.
    pub fn init(cx: &mut App) {
        cx.set_global(Theme::default());
    }

    /// Borrow the global theme.
    #[inline(always)]
    pub fn global(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }

    /// Mutably borrow the global theme.
    #[inline(always)]
    pub fn global_mut(cx: &mut App) -> &mut Theme {
        cx.global_mut::<Theme>()
    }

    /// Replace the global theme.
    pub fn set(theme: Theme, cx: &mut App) {
        *cx.global_mut::<Theme>() = theme;
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_palette("Default Dark", Base16Palette::default())
    }
}

/// Convenient access to the active theme.
pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    #[inline(always)]
    fn theme(&self) -> &Theme {
        Theme::global(self)
    }
}

/// On-disk representation of a theme (`theme.toml`).
///
/// Stores the palette and the overrides, never the resolved tokens - the
/// ~50 resolved values are an implementation detail that would go stale the
/// moment the derivation changes.
///
/// ```toml
/// name = "Catppuccin Mocha"
/// font_size = 13.0
///
/// [base16]
/// base00 = "#1e1e2e"
/// # ... through base0f
///
/// [colors]              # optional; anything omitted falls through
/// accent = "#f38ba8"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StoredTheme {
    pub name: String,
    pub font_size: f32,
    pub base16: Base16Palette,
    #[serde(skip_serializing_if = "ThemeColorsRefinement::is_empty")]
    pub colors: ThemeColorsRefinement,
}

impl Default for StoredTheme {
    fn default() -> Self {
        Self::from_theme(&Theme::default())
    }
}

impl StoredTheme {
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            name: theme.name.to_string(),
            font_size: theme.font_size.into(),
            base16: theme.palette,
            colors: theme.overrides,
        }
    }

    pub fn into_theme(self) -> Theme {
        let mut theme = Theme::from_palette(self.name, self.base16);
        theme.font_size = px(self.font_size);
        theme.set_overrides(self.colors);
        theme
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overrides_win_over_the_palette_derivation() {
        let mut theme = Theme::default();
        let derived_accent = theme.colors().accent;

        let overrides = ThemeColorsRefinement {
            accent: Some(gpui::hsla(0.0, 1.0, 0.5, 1.0)),
            ..Default::default()
        };
        theme.set_overrides(overrides);

        assert_ne!(theme.colors().accent, derived_accent);
        // Untouched tokens still come from the palette.
        assert_eq!(theme.colors().background, theme.palette.base00);
    }

    /// The point of keeping palette and overrides separate: changing
    /// scheme must not silently discard what the user pinned.
    #[test]
    fn overrides_survive_a_palette_swap() {
        let mut theme = Theme::default();
        let pinned = gpui::hsla(0.0, 1.0, 0.5, 1.0);
        theme.set_overrides(ThemeColorsRefinement {
            accent: Some(pinned),
            ..Default::default()
        });

        let mut light = Base16Palette::default();
        std::mem::swap(&mut light.base00, &mut light.base07);
        theme.set_palette("Swapped", light);

        assert_eq!(theme.colors().accent, pinned);
        assert_eq!(theme.appearance, Appearance::Light);
        assert_eq!(theme.colors().background, light.base00);
    }

    #[test]
    fn stored_theme_round_trips_through_toml() {
        let mut theme = Theme::default();
        theme.font_size = px(15.0);
        theme.set_overrides(ThemeColorsRefinement {
            text_accent: Some(gpui::hsla(0.5, 1.0, 0.5, 1.0)),
            ..Default::default()
        });

        let encoded = toml::to_string_pretty(&StoredTheme::from_theme(&theme)).unwrap();
        let decoded: StoredTheme = toml::from_str(&encoded).unwrap();
        let restored = decoded.into_theme();

        assert_eq!(restored.name, theme.name);
        assert_eq!(restored.font_size, theme.font_size);
        assert_eq!(restored.palette, theme.palette);
        assert_eq!(restored.colors, theme.colors);
    }

    /// A theme with no overrides should not litter the file with 50 nulls.
    #[test]
    fn a_plain_theme_serializes_without_a_colors_table() {
        let encoded = toml::to_string_pretty(&StoredTheme::from_theme(&Theme::default())).unwrap();
        assert!(!encoded.contains("[colors]"), "{encoded}");
        assert!(encoded.contains("base00"), "{encoded}");
    }

    #[test]
    fn font_size_is_independent_of_the_palette() {
        let mut theme = Theme::default();
        theme.font_size = px(18.0);
        theme.set_palette("Other", Base16Palette::default());
        assert_eq!(theme.font_size, px(18.0));
    }
}
