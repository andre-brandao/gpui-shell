//! Built-in theme schemes.
//!
//! Each scheme provides a complete `Theme` with all color groups defined.
//! In the future, schemes will also be loaded from disk and network.

use gpui::{Hsla, px, rgba};

use super::{
    AccentColors, BgColors, BorderColors, InteractiveColors, StatusColors, TextColors, Theme,
};

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
    vec![default_scheme(), nord_scheme(), catppuccin_mocha_scheme()]
}

fn default_scheme() -> ThemeScheme {
    ThemeScheme {
        name: "Default",
        description: "Dark theme with blue accents",
        theme: Theme::default(),
    }
}

fn nord_scheme() -> ThemeScheme {
    ThemeScheme {
        name: "Nord",
        description: "Arctic, north-bluish palette",
        theme: Theme {
            bg: BgColors {
                primary: rgba(0x2e3440ff).into(),
                secondary: rgba(0x3b4252ff).into(),
                tertiary: rgba(0x434c5eff).into(),
                elevated: rgba(0x4c566aff).into(),
            },
            text: TextColors {
                primary: rgba(0xeceff4ff).into(),
                secondary: rgba(0xd8dee9ff).into(),
                muted: rgba(0x81a1c1ff).into(),
                disabled: rgba(0x616e88ff).into(),
                placeholder: rgba(0x616e88ff).into(),
            },
            border: BorderColors {
                default: rgba(0x4c566aff).into(),
                subtle: rgba(0x434c5eff).into(),
                focused: rgba(0x88c0d0ff).into(),
            },
            accent: AccentColors {
                primary: rgba(0x88c0d0ff).into(),
                selection: rgba(0x434c5eff).into(),
                hover: rgba(0x81a1c1ff).into(),
            },
            status: StatusColors {
                success: rgba(0xa3be8cff).into(),
                warning: rgba(0xebcb8bff).into(),
                error: rgba(0xbf616aff).into(),
                info: rgba(0x5e81acff).into(),
            },
            interactive: InteractiveColors {
                default: rgba(0x3b4252ff).into(),
                hover: rgba(0x434c5eff).into(),
                active: rgba(0x4c566aff).into(),
                toggle_on: rgba(0x88c0d0ff).into(),
                toggle_on_hover: rgba(0x81a1c1ff).into(),
            },
            radius: px(6.0),
            radius_lg: px(8.0),
            transparent: Hsla::transparent_black(),
        },
    }
}

fn catppuccin_mocha_scheme() -> ThemeScheme {
    ThemeScheme {
        name: "Catppuccin Mocha",
        description: "Warm dark theme with pastel accents",
        theme: Theme {
            bg: BgColors {
                primary: rgba(0x1e1e2eff).into(),
                secondary: rgba(0x181825ff).into(),
                tertiary: rgba(0x313244ff).into(),
                elevated: rgba(0x45475aff).into(),
            },
            text: TextColors {
                primary: rgba(0xcdd6f4ff).into(),
                secondary: rgba(0xbac2deff).into(),
                muted: rgba(0xa6adc8ff).into(),
                disabled: rgba(0x6c7086ff).into(),
                placeholder: rgba(0x6c7086ff).into(),
            },
            border: BorderColors {
                default: rgba(0x45475aff).into(),
                subtle: rgba(0x313244ff).into(),
                focused: rgba(0x89b4faff).into(),
            },
            accent: AccentColors {
                primary: rgba(0x89b4faff).into(),
                selection: rgba(0x313244ff).into(),
                hover: rgba(0x74c7ecff).into(),
            },
            status: StatusColors {
                success: rgba(0xa6e3a1ff).into(),
                warning: rgba(0xf9e2afff).into(),
                error: rgba(0xf38ba8ff).into(),
                info: rgba(0x89dcebff).into(),
            },
            interactive: InteractiveColors {
                default: rgba(0x313244ff).into(),
                hover: rgba(0x45475aff).into(),
                active: rgba(0x585b70ff).into(),
                toggle_on: rgba(0x89b4faff).into(),
                toggle_on_hover: rgba(0x74c7ecff).into(),
            },
            radius: px(6.0),
            radius_lg: px(8.0),
            transparent: Hsla::transparent_black(),
        },
    }
}
