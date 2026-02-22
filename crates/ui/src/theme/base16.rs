//! Base16 color palette to Theme conversion.
//!
//! Converts a set of 16 hex color strings (the Base16 standard) into
//! the application's `Theme` struct.

use std::path::Path;
use std::process::Command;

use gpui::{Hsla, px};

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

    /// Generate a Base16 theme from a wallpaper using matugen.
    ///
    /// Runs matugen CLI to extract colors from the image and parse the result.
    ///
    /// # Arguments
    /// - `wallpaper_path`: Path to the wallpaper image
    /// - `mode`: "dark" or "light"
    /// - `scheme_type`: e.g. "scheme-tonal-spot", "scheme-vibrant", etc.
    /// - `source_color_index`: 0-4, where 0 is most dominant color
    ///
    /// # Returns
    /// - `Ok(Theme)` on success
    /// - `Err` if matugen fails or output cannot be parsed
    pub fn generate_from_wallpaper(
        wallpaper_path: impl AsRef<Path>,
        mode: &str,
        scheme_type: &str,
        source_color_index: usize,
    ) -> anyhow::Result<Theme> {
        let path = wallpaper_path.as_ref();

        // Run matugen to generate base16 colors
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
            .map_err(|e| anyhow::anyhow!("Failed to run matugen: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Matugen failed: {}", stderr));
        }

        // Parse the JSON output
        let json_str = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse matugen output: {}", e))?;

        // Extract base16 colors from the JSON
        // Matugen outputs colors in structure: base16.baseXX.{dark,light}.color
        let base16 = json
            .get("base16")
            .ok_or_else(|| anyhow::anyhow!("Missing base16 colors in matugen output"))?;

        // Helper to extract color for the given mode
        let get_color = |base_key: &str| -> &str {
            base16
                .get(base_key)
                .and_then(|b| b.get(mode))
                .and_then(|m| m.get("color"))
                .and_then(|v| v.as_str())
                .unwrap_or("#000000")
        };

        // Parse the 16 base colors
        let base_colors: [&str; 16] = [
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
        ];

        let base16 = Self::from_hex(&base_colors)?;
        Ok(base16.to_theme())
    }
}
