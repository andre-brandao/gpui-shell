use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use ui::{
    AccentColors, BgColors, BorderColors, Colorize as _, FontSizes, InteractiveColors,
    StatusColors, TextColors, Theme,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredTheme {
    bg: BgSection,
    text: TextSection,
    border: BorderSection,
    accent: AccentSection,
    status: StatusSection,
    interactive: InteractiveSection,
    font_size_base: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BgSection {
    primary: String,
    secondary: String,
    tertiary: String,
    elevated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TextSection {
    primary: String,
    secondary: String,
    muted: String,
    disabled: String,
    placeholder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BorderSection {
    default: String,
    subtle: String,
    focused: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccentSection {
    primary: String,
    selection: String,
    hover: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusSection {
    success: String,
    warning: String,
    error: String,
    info: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InteractiveSection {
    default: String,
    hover: String,
    active: String,
    toggle_on: String,
    toggle_on_hover: String,
}

impl StoredTheme {
    pub(crate) fn from_theme(theme: &Theme) -> Self {
        Self {
            bg: BgSection {
                primary: hsla_to_hex(theme.bg.primary),
                secondary: hsla_to_hex(theme.bg.secondary),
                tertiary: hsla_to_hex(theme.bg.tertiary),
                elevated: hsla_to_hex(theme.bg.elevated),
            },
            text: TextSection {
                primary: hsla_to_hex(theme.text.primary),
                secondary: hsla_to_hex(theme.text.secondary),
                muted: hsla_to_hex(theme.text.muted),
                disabled: hsla_to_hex(theme.text.disabled),
                placeholder: hsla_to_hex(theme.text.placeholder),
            },
            border: BorderSection {
                default: hsla_to_hex(theme.border.default),
                subtle: hsla_to_hex(theme.border.subtle),
                focused: hsla_to_hex(theme.border.focused),
            },
            accent: AccentSection {
                primary: hsla_to_hex(theme.accent.primary),
                selection: hsla_to_hex(theme.accent.selection),
                hover: hsla_to_hex(theme.accent.hover),
            },
            status: StatusSection {
                success: hsla_to_hex(theme.status.success),
                warning: hsla_to_hex(theme.status.warning),
                error: hsla_to_hex(theme.status.error),
                info: hsla_to_hex(theme.status.info),
            },
            interactive: InteractiveSection {
                default: hsla_to_hex(theme.interactive.default),
                hover: hsla_to_hex(theme.interactive.hover),
                active: hsla_to_hex(theme.interactive.active),
                toggle_on: hsla_to_hex(theme.interactive.toggle_on),
                toggle_on_hover: hsla_to_hex(theme.interactive.toggle_on_hover),
            },
            font_size_base: theme.font_sizes.base_value(),
        }
    }

    pub(crate) fn to_theme(&self) -> anyhow::Result<Theme> {
        Ok(Theme {
            bg: BgColors {
                primary: hex_to_hsla(&self.bg.primary)?,
                secondary: hex_to_hsla(&self.bg.secondary)?,
                tertiary: hex_to_hsla(&self.bg.tertiary)?,
                elevated: hex_to_hsla(&self.bg.elevated)?,
            },
            text: TextColors {
                primary: hex_to_hsla(&self.text.primary)?,
                secondary: hex_to_hsla(&self.text.secondary)?,
                muted: hex_to_hsla(&self.text.muted)?,
                disabled: hex_to_hsla(&self.text.disabled)?,
                placeholder: hex_to_hsla(&self.text.placeholder)?,
            },
            border: BorderColors {
                default: hex_to_hsla(&self.border.default)?,
                subtle: hex_to_hsla(&self.border.subtle)?,
                focused: hex_to_hsla(&self.border.focused)?,
            },
            accent: AccentColors {
                primary: hex_to_hsla(&self.accent.primary)?,
                selection: hex_to_hsla(&self.accent.selection)?,
                hover: hex_to_hsla(&self.accent.hover)?,
            },
            status: StatusColors {
                success: hex_to_hsla(&self.status.success)?,
                warning: hex_to_hsla(&self.status.warning)?,
                error: hex_to_hsla(&self.status.error)?,
                info: hex_to_hsla(&self.status.info)?,
            },
            interactive: InteractiveColors {
                default: hex_to_hsla(&self.interactive.default)?,
                hover: hex_to_hsla(&self.interactive.hover)?,
                active: hex_to_hsla(&self.interactive.active)?,
                toggle_on: hex_to_hsla(&self.interactive.toggle_on)?,
                toggle_on_hover: hex_to_hsla(&self.interactive.toggle_on_hover)?,
            },
            font_sizes: FontSizes::new(self.font_size_base),
            ..Theme::default()
        })
    }
}

fn hsla_to_hex(color: gpui::Hsla) -> String {
    color.to_hex()
}

fn hex_to_hsla(hex: &str) -> anyhow::Result<gpui::Hsla> {
    gpui::Hsla::parse_hex(hex.trim())
        .with_context(|| format!("Invalid color '{hex}': expected #RRGGBB or #RRGGBBAA"))
}
