//! Base16 palette → [`ThemeColors`] derivation.
//!
//! Base16 is the shell's *input* format: 16 colors, coming from a Tinted
//! Theming scheme, from Stylix, from matugen run against the wallpaper, or
//! written by hand. [`ThemeColors`] is the *output*: the ~50 semantic
//! tokens components actually read.
//!
//! 16 inputs cannot fill 50 slots directly, so the slots Base16 does not
//! name are derived - almost always by laying a translucent foreground or
//! accent over a surface. Deriving rather than hard-coding is what keeps
//! matugen output (arbitrary hues, arbitrary contrast) usable: an
//! `alpha(0.06)` overlay reads correctly on any palette, light or dark.
//!
//! # Slot mapping
//!
//! Following the Base16 styling guidelines:
//!
//! | Slot | Role | Used for |
//! |------|------|----------|
//! | `base00` | default background | `background` |
//! | `base01` | lighter background | `surface_background`, resting elements |
//! | `base02` | selection background | `elevated_surface_background`, `border`, hover |
//! | `base03` | comments, invisibles | disabled/placeholder foreground, active elements |
//! | `base04` | dark foreground | `text_muted`, `icon_muted` |
//! | `base05` | default foreground | `text`, `icon` |
//! | `base08` | red | `status.error` |
//! | `base09` | orange | `status.warning` |
//! | `base0B` | green | `status.success` |
//! | `base0C` | cyan | `status.info` |
//! | `base0D` | blue | `accent`, focus, selection |
//!
//! `base06`, `base07`, `base0A`, `base0E` and `base0F` are carried on the
//! palette (theme authors and preview swatches want the full 16) but are
//! not mapped to a token - nothing in the shell needs a second light
//! foreground or a "deprecated" brown.

use std::path::Path;
use std::process::Command;

use gpui::Hsla;
use serde::{Deserialize, Serialize};

use super::color_string;
use super::colors::{StatusColors, ThemeColors};
use super::{Appearance, Theme};

/// Fully transparent, for the token slots that are meant to be invisible.
const TRANSPARENT: Hsla = Hsla {
    h: 0.0,
    s: 0.0,
    l: 0.0,
    a: 0.0,
};

/// Alpha of a status color when used as a surface fill.
const STATUS_BACKGROUND_ALPHA: f32 = 0.12;
/// Alpha of a status color when used as that surface's border.
const STATUS_BORDER_ALPHA: f32 = 0.30;

/// A Base16 color palette: the 16 slots, parsed into [`Hsla`].
///
/// Serializes to (and from) the shape every Base16 tool already writes, so
/// a `theme.toml` reads like any other scheme file:
///
/// ```toml
/// [base16]
/// base00 = "#181818"
/// base01 = "#282828"
/// # ...
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Base16Palette {
    #[serde(with = "super::color_string::required")]
    pub base00: Hsla,
    #[serde(with = "super::color_string::required")]
    pub base01: Hsla,
    #[serde(with = "super::color_string::required")]
    pub base02: Hsla,
    #[serde(with = "super::color_string::required")]
    pub base03: Hsla,
    #[serde(with = "super::color_string::required")]
    pub base04: Hsla,
    #[serde(with = "super::color_string::required")]
    pub base05: Hsla,
    #[serde(with = "super::color_string::required")]
    pub base06: Hsla,
    #[serde(with = "super::color_string::required")]
    pub base07: Hsla,
    #[serde(with = "super::color_string::required")]
    pub base08: Hsla,
    #[serde(with = "super::color_string::required")]
    pub base09: Hsla,
    #[serde(with = "super::color_string::required")]
    pub base0a: Hsla,
    #[serde(with = "super::color_string::required")]
    pub base0b: Hsla,
    #[serde(with = "super::color_string::required")]
    pub base0c: Hsla,
    #[serde(with = "super::color_string::required")]
    pub base0d: Hsla,
    #[serde(with = "super::color_string::required")]
    pub base0e: Hsla,
    #[serde(with = "super::color_string::required")]
    pub base0f: Hsla,
}

impl Base16Palette {
    /// Parse 16 color strings (`base00` through `base0F`, in order).
    ///
    /// Each entry accepts any form [`color_string`] understands - `#1e1e2e`,
    /// bare `1e1e2e`, `rgb(...)`, `hsl(...)`, or `oklch(...)`.
    pub fn from_hex(colors: &[&str; 16]) -> anyhow::Result<Self> {
        let parse = |idx: usize| -> anyhow::Result<Hsla> {
            color_string::parse(colors[idx]).map_err(|e| anyhow::anyhow!("base{idx:02x}: {e}"))
        };
        Ok(Self {
            base00: parse(0)?,
            base01: parse(1)?,
            base02: parse(2)?,
            base03: parse(3)?,
            base04: parse(4)?,
            base05: parse(5)?,
            base06: parse(6)?,
            base07: parse(7)?,
            base08: parse(8)?,
            base09: parse(9)?,
            base0a: parse(10)?,
            base0b: parse(11)?,
            base0c: parse(12)?,
            base0d: parse(13)?,
            base0e: parse(14)?,
            base0f: parse(15)?,
        })
    }

    /// Whether this palette is a light or a dark scheme, judged by the
    /// lightness of its default background.
    ///
    /// Base16 schemes are self-describing this way - a light scheme ships
    /// a light `base00` - so nothing needs inverting downstream.
    pub fn appearance(&self) -> Appearance {
        if self.base00.l < 0.5 {
            Appearance::Dark
        } else {
            Appearance::Light
        }
    }

    /// The 16 slots in order, for preview swatches.
    pub fn swatches(&self) -> [Hsla; 16] {
        [
            self.base00,
            self.base01,
            self.base02,
            self.base03,
            self.base04,
            self.base05,
            self.base06,
            self.base07,
            self.base08,
            self.base09,
            self.base0a,
            self.base0b,
            self.base0c,
            self.base0d,
            self.base0e,
            self.base0f,
        ]
    }

    /// Expand this palette into the full semantic token set.
    pub fn into_colors(self) -> ThemeColors {
        let fg = self.base05;
        let accent = self.base0d;

        ThemeColors {
            // Surfaces climb base00 → base01 → base02.
            background: self.base00,
            surface_background: self.base01,
            elevated_surface_background: self.base02,

            // `border_variant` is the *subtler* of the two - it separates
            // related content, so it sits one step closer to the surface
            // than `border` does.
            border: self.base02,
            border_variant: self.base01,
            border_focused: accent,
            border_selected: accent,
            border_disabled: self.base01,
            border_transparent: TRANSPARENT,

            // Foreground ramp base05 → base04 → base03.
            text: fg,
            text_muted: self.base04,
            text_placeholder: self.base03,
            text_disabled: self.base03,
            text_accent: accent,

            icon: fg,
            icon_muted: self.base04,
            icon_disabled: self.base03,
            icon_accent: accent,

            // Filled controls reuse the surface ramp so they read as
            // raised chips against the background.
            element_background: self.base01,
            element_hover: self.base02,
            element_active: self.base03,
            element_selected: accent.alpha(0.20),
            element_disabled: self.base01,

            // Ghost controls tint with the foreground, so the same alphas
            // darken on a light scheme and lighten on a dark one.
            ghost_element_background: TRANSPARENT,
            ghost_element_hover: fg.alpha(0.06),
            ghost_element_active: fg.alpha(0.10),
            ghost_element_selected: accent.alpha(0.15),
            ghost_element_disabled: TRANSPARENT,

            status: self.status_colors(),

            accent,
        }
    }

    fn status_colors(self) -> StatusColors {
        // Each status flavor gets its foreground straight from the palette,
        // then a surface fill and a border derived by alpha.
        let [
            (info, info_background, info_border),
            (success, success_background, success_border),
            (warning, warning_background, warning_border),
            (error, error_background, error_border),
            (hint, hint_background, hint_border),
            (hidden, hidden_background, hidden_border),
            (ignored, ignored_background, ignored_border),
        ] = [
            self.base0c, // cyan
            self.base0b, // green
            self.base09, // orange
            self.base08, // red
            self.base0d, // blue
            self.base03, // comment grey
            self.base03,
        ]
        .map(status_triplet);

        StatusColors {
            info,
            info_background,
            info_border,
            success,
            success_background,
            success_border,
            warning,
            warning_background,
            warning_border,
            error,
            error_background,
            error_border,
            hint,
            hint_background,
            hint_border,
            hidden,
            hidden_background,
            hidden_border,
            ignored,
            ignored_background,
            ignored_border,
        }
    }

    /// Build a complete [`Theme`] from this palette, at the default font
    /// size. Callers that carry a user-configured size should set
    /// [`Theme::font_size`] afterwards.
    pub fn into_theme(self, name: impl Into<gpui::SharedString>) -> Theme {
        Theme::from_palette(name, self)
    }

    /// Generate a Base16 palette from a wallpaper using matugen.
    ///
    /// # Arguments
    /// - `wallpaper_path`: path to the wallpaper image
    /// - `mode`: `"dark"` or `"light"`
    /// - `scheme_type`: e.g. `"scheme-tonal-spot"`, `"scheme-vibrant"`
    /// - `source_color_index`: 0-4, where 0 is the most dominant color
    pub fn generate_from_wallpaper(
        wallpaper_path: impl AsRef<Path>,
        mode: &str,
        scheme_type: &str,
        source_color_index: usize,
    ) -> anyhow::Result<Self> {
        let path = wallpaper_path.as_ref();

        let output = Command::new("matugen")
            .args([
                "image",
                &path.to_string_lossy(),
                "--mode",
                mode,
                "--type",
                scheme_type,
                "--base16-backend",
                "wal",
                "--source-color-index",
                &source_color_index.to_string(),
                "--json",
                "hex",
            ])
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run matugen: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Matugen failed: {stderr}"));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse matugen output: {e}"))?;

        // Matugen nests colors as base16.baseXX.{dark,light}.color
        let base16 = json
            .get("base16")
            .ok_or_else(|| anyhow::anyhow!("Missing base16 colors in matugen output"))?;

        let get_color = |base_key: &str| -> &str {
            base16
                .get(base_key)
                .and_then(|b| b.get(mode))
                .and_then(|m| m.get("color"))
                .and_then(|v| v.as_str())
                .unwrap_or("#000000")
        };

        Self::from_hex(&[
            get_color("base00"),
            get_color("base01"),
            get_color("base02"),
            get_color("base03"),
            get_color("base04"),
            get_color("base05"),
            get_color("base06"),
            get_color("base07"),
            get_color("base08"),
            get_color("base09"),
            get_color("base0a"),
            get_color("base0b"),
            get_color("base0c"),
            get_color("base0d"),
            get_color("base0e"),
            get_color("base0f"),
        ])
    }
}

fn status_triplet(fg: Hsla) -> (Hsla, Hsla, Hsla) {
    (
        fg,
        fg.alpha(STATUS_BACKGROUND_ALPHA),
        fg.alpha(STATUS_BORDER_ALPHA),
    )
}

impl Default for Base16Palette {
    /// The canonical `base16-default-dark` scheme, used when no theme file
    /// exists yet and as the fallback when one fails to parse.
    fn default() -> Self {
        Self::from_hex(&[
            "#181818", "#282828", "#383838", "#585858", "#b8b8b8", "#d8d8d8", "#e8e8e8", "#f8f8f8",
            "#ab4642", "#dc9656", "#f7ca88", "#a1b56c", "#86c1b9", "#7cafc2", "#ba8baf", "#a16946",
        ])
        .expect("the built-in default palette must parse")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_palette_parses_and_is_dark() {
        let palette = Base16Palette::default();
        assert_eq!(palette.appearance(), Appearance::Dark);
        assert_eq!(palette.swatches().len(), 16);
    }

    #[test]
    fn appearance_follows_background_lightness() {
        let mut light = Base16Palette::default();
        std::mem::swap(&mut light.base00, &mut light.base07);
        assert_eq!(light.appearance(), Appearance::Light);
    }

    /// A "variant" border must be subtler than the default one, i.e. closer
    /// to the surface it sits on. Easy to invert by accident: `base07` is the
    /// lightest slot, so mapping it to `border.subtle` makes it the loudest.
    #[test]
    fn border_variant_is_subtler_than_border() {
        let colors = Base16Palette::default().into_colors();
        let contrast = |c: Hsla| (c.l - colors.surface_background.l).abs();
        assert!(
            contrast(colors.border_variant) < contrast(colors.border),
            "border_variant must sit closer to the surface than border"
        );
    }

    /// The other bug: `text` and `text_muted` both resolved to `base05`,
    /// so "muted" text was not actually muted.
    #[test]
    fn foreground_ramp_is_strictly_descending() {
        let colors = Base16Palette::default().into_colors();
        // Dark scheme: each step down the ramp gets darker.
        assert!(colors.text.l > colors.text_muted.l);
        assert!(colors.text_muted.l > colors.text_disabled.l);
    }

    #[test]
    fn status_surfaces_are_translucent_versions_of_their_foreground() {
        let colors = Base16Palette::default().into_colors();
        for (fg, bg, border) in [
            (
                colors.status.error,
                colors.status.error_background,
                colors.status.error_border,
            ),
            (
                colors.status.success,
                colors.status.success_background,
                colors.status.success_border,
            ),
        ] {
            assert_eq!(fg.a, 1.0);
            assert_eq!(bg.h, fg.h);
            assert!(bg.a < border.a && border.a < fg.a);
        }
    }

    #[test]
    fn ghost_states_are_transparent_at_rest_and_tint_when_interactive() {
        let colors = Base16Palette::default().into_colors();
        assert_eq!(colors.ghost_element_background.a, 0.0);
        assert!(colors.ghost_element_hover.a > 0.0);
        assert!(colors.ghost_element_active.a > colors.ghost_element_hover.a);
    }

    #[test]
    fn from_hex_reports_which_slot_failed() {
        let mut colors = ["#000000"; 16];
        colors[9] = "not-a-color";
        let err = Base16Palette::from_hex(&colors).unwrap_err().to_string();
        assert!(err.contains("base09"), "unhelpful error: {err}");
    }
}
