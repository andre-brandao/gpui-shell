use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use ui::{
    AccentColors, BgColors, BorderColors, FontSizes, InteractiveColors, StatusColors, TextColors,
    Theme,
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
    let (r, g, b, a) = hsla_to_rgba8(color);
    if a == 255 {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
    }
}

fn hex_to_hsla(hex: &str) -> anyhow::Result<gpui::Hsla> {
    let trimmed = hex.trim();
    let raw = trimmed.strip_prefix('#').unwrap_or(trimmed);

    let (r, g, b, a) = match raw.len() {
        6 => (
            u8::from_str_radix(&raw[0..2], 16)?,
            u8::from_str_radix(&raw[2..4], 16)?,
            u8::from_str_radix(&raw[4..6], 16)?,
            255,
        ),
        8 => (
            u8::from_str_radix(&raw[0..2], 16)?,
            u8::from_str_radix(&raw[2..4], 16)?,
            u8::from_str_radix(&raw[4..6], 16)?,
            u8::from_str_radix(&raw[6..8], 16)?,
        ),
        _ => {
            return Err(anyhow!(
                "Invalid color '{}': expected #RRGGBB or #RRGGBBAA",
                hex
            ));
        }
    };

    Ok(rgba8_to_hsla(r, g, b, a))
}

fn hsla_to_rgba8(color: gpui::Hsla) -> (u8, u8, u8, u8) {
    let h = wrap01(color.h);
    let s = clamp01(color.s);
    let l = clamp01(color.l);
    let a = clamp01(color.a);

    let (r, g, b) = if s == 0.0 {
        (l, l, l)
    } else {
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - (l * s)
        };
        let p = 2.0 * l - q;
        (
            hue_to_rgb(p, q, h + 1.0 / 3.0),
            hue_to_rgb(p, q, h),
            hue_to_rgb(p, q, h - 1.0 / 3.0),
        )
    };

    (
        float01_to_u8(r),
        float01_to_u8(g),
        float01_to_u8(b),
        float01_to_u8(a),
    )
}

fn rgba8_to_hsla(r: u8, g: u8, b: u8, a: u8) -> gpui::Hsla {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;
    let af = a as f32 / 255.0;

    let max = rf.max(gf.max(bf));
    let min = rf.min(gf.min(bf));
    let mut h = 0.0;
    let l = (max + min) / 2.0;

    let d = max - min;
    let s = if d == 0.0 {
        0.0
    } else {
        d / (1.0 - (2.0 * l - 1.0).abs())
    };

    if d != 0.0 {
        if max == rf {
            h = ((gf - bf) / d) % 6.0;
        } else if max == gf {
            h = ((bf - rf) / d) + 2.0;
        } else {
            h = ((rf - gf) / d) + 4.0;
        }
        h /= 6.0;
        if h < 0.0 {
            h += 1.0;
        }
    }

    gpui::Hsla {
        h,
        s: clamp01(s),
        l: clamp01(l),
        a: clamp01(af),
    }
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }

    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 1.0 / 2.0 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn wrap01(value: f32) -> f32 {
    let mut v = value % 1.0;
    if v < 0.0 {
        v += 1.0;
    }
    v
}

fn float01_to_u8(value: f32) -> u8 {
    (clamp01(value) * 255.0).round() as u8
}
