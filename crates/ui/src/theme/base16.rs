//! Base16 color palette to Theme conversion.
//!
//! Converts a set of 16 hex color strings (the Base16 standard) into
//! the application's `Theme` struct.

use gpui::{px, Hsla};

use super::{
    AccentColors, BgColors, BorderColors, Colorize, FontSizes, InteractiveColors, StatusColors,
    TextColors, Theme,
};

/// Parsed Base16 colors ready for Theme conversion.
///
/// Construct via `Base16Colors::from_hex(...)` then call `to_theme()`.
pub struct Base16Colors {
    pub base00: Hsla,
    pub base01: Hsla,
    pub base02: Hsla,
    pub base03: Hsla,
    pub base04: Hsla,
    pub base05: Hsla,
    pub base06: Hsla,
    pub base07: Hsla,
    pub base08: Hsla,
    pub base09: Hsla,
    pub base0a: Hsla,
    pub base0b: Hsla,
    pub base0c: Hsla,
    pub base0d: Hsla,
    pub base0e: Hsla,
    pub base0f: Hsla,
}

impl Base16Colors {
    /// Parse 16 hex strings (base00 through base0F) into Base16Colors.
    ///
    /// Each string should be like "#1e1e2e" or "1e1e2e".
    /// The array must contain exactly 16 entries in order: base00..base0F.
    pub fn from_hex(colors: &[&str; 16]) -> anyhow::Result<Self> {
        Ok(Self {
            base00: Hsla::parse_hex(colors[0])?,
            base01: Hsla::parse_hex(colors[1])?,
            base02: Hsla::parse_hex(colors[2])?,
            base03: Hsla::parse_hex(colors[3])?,
            base04: Hsla::parse_hex(colors[4])?,
            base05: Hsla::parse_hex(colors[5])?,
            base06: Hsla::parse_hex(colors[6])?,
            base07: Hsla::parse_hex(colors[7])?,
            base08: Hsla::parse_hex(colors[8])?,
            base09: Hsla::parse_hex(colors[9])?,
            base0a: Hsla::parse_hex(colors[10])?,
            base0b: Hsla::parse_hex(colors[11])?,
            base0c: Hsla::parse_hex(colors[12])?,
            base0d: Hsla::parse_hex(colors[13])?,
            base0e: Hsla::parse_hex(colors[14])?,
            base0f: Hsla::parse_hex(colors[15])?,
        })
    }

    /// Convert this Base16 palette into a full Theme.
    ///
    /// Mapping follows Base16 semantics:
    /// - base00..base07: backgrounds and foregrounds (dark to light)
    /// - base08..base0F: accent colors (red, orange, yellow, green, cyan, blue, purple, brown)
    pub fn to_theme(&self) -> Theme {
        Theme {
            bg: BgColors {
                primary: self.base00,
                secondary: self.base01,
                tertiary: self.base02,
                elevated: self.base06,
            },
            text: TextColors {
                primary: self.base05,
                secondary: self.base05,
                muted: self.base04,
                disabled: self.base03,
                placeholder: self.base03,
            },
            border: BorderColors {
                default: self.base04,
                subtle: self.base07,
                focused: self.base0d,
            },
            accent: AccentColors {
                primary: self.base0d,
                selection: self.base02,
                hover: self.base0a,
            },
            status: StatusColors {
                success: self.base0b,
                warning: self.base09,
                error: self.base08,
                info: self.base0c,
            },
            interactive: InteractiveColors {
                default: self.base01,
                hover: self.base02,
                active: self.base03,
                toggle_on: self.base0d,
                toggle_on_hover: self.base0e,
            },
            radius: px(6.0),
            radius_lg: px(8.0),
            transparent: Hsla::transparent_black(),
            font_sizes: FontSizes::default(),
        }
    }
}
