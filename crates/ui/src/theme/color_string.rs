//! Color string parsing for theme files, plus the OKLCH → sRGB math.
//!
//! Theme files accept four CSS color forms, parsed here into [`Hsla`]:
//!
//! * Hex — `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`
//! * `rgb()` / `rgba()` — `rgb(255 0 0)`, `rgb(255, 0, 0, 0.5)`,
//!   `rgb(255 0 0 / 50%)`, with channels as ints (`0..=255`) or percentages
//! * `hsl()` / `hsla()` — `hsl(210 80% 50%)`, legacy commas, slash alpha
//! * `oklch()` — `oklch(0.18 0.004 70)`, slash alpha; lightness as `0..1`
//!   float or percentage, hue in degrees
//!
//! A narrow subset of CSS Color Module 4: no `calc()`, `none`, relative
//! colors, or `turn` / `rad` / `grad` hue units. Serialization always emits
//! `#rrggbbaa`.

use gpui::{Hsla, Rgba};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Parse a CSS color string into an [`Hsla`].
pub(crate) fn parse(input: &str) -> Result<Hsla, String> {
    let s = input.trim();
    if let Some(rest) = s.strip_prefix('#') {
        return parse_hex(rest);
    }
    if let Some(inner) = strip_fn(s, "rgba") {
        return parse_rgb(inner);
    }
    if let Some(inner) = strip_fn(s, "rgb") {
        return parse_rgb(inner);
    }
    if let Some(inner) = strip_fn(s, "hsla") {
        return parse_hsl(inner);
    }
    if let Some(inner) = strip_fn(s, "hsl") {
        return parse_hsl(inner);
    }
    if let Some(inner) = strip_fn(s, "oklch") {
        return parse_oklch(inner);
    }
    // Bare hex without the `#` - Base16 YAML schemes in the wild write both
    // `#1e1e2e` and `1e1e2e`.
    parse_hex(s)
}

fn strip_fn<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let rest = s.strip_prefix(name)?.trim_start();
    let inner = rest.strip_prefix('(')?.strip_suffix(')')?;
    Some(inner.trim())
}

fn parse_hex(s: &str) -> Result<Hsla, String> {
    if !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("unknown color syntax: {s:?}"));
    }
    let rgba = match s.len() {
        3 | 4 => {
            let expand = |idx: usize| -> Result<u8, String> {
                let h = u8::from_str_radix(&s[idx..idx + 1], 16)
                    .map_err(|e| format!("bad hex digit {:?}: {e}", &s[idx..idx + 1]))?;
                Ok(h * 17)
            };
            let r = expand(0)?;
            let g = expand(1)?;
            let b = expand(2)?;
            let a = if s.len() == 4 { expand(3)? } else { 255 };
            (r, g, b, a)
        }
        6 | 8 => {
            let pair = |idx: usize| -> Result<u8, String> {
                u8::from_str_radix(&s[idx..idx + 2], 16)
                    .map_err(|e| format!("bad hex pair {:?}: {e}", &s[idx..idx + 2]))
            };
            let r = pair(0)?;
            let g = pair(2)?;
            let b = pair(4)?;
            let a = if s.len() == 8 { pair(6)? } else { 255 };
            (r, g, b, a)
        }
        n => return Err(format!("hex color must have 3/4/6/8 digits, got {n}")),
    };
    let (r, g, b, a) = rgba;
    Ok(Hsla::from(Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: a as f32 / 255.0,
    }))
}

/// Split `func(...)` inner into (channel parts, optional alpha).
fn split_args(s: &str) -> (Vec<&str>, Option<&str>) {
    if let Some((main, alpha)) = s.split_once('/') {
        let parts = main.split_whitespace().collect::<Vec<_>>();
        return (parts, Some(alpha.trim()));
    }
    if s.contains(',') {
        let parts: Vec<&str> = s.split(',').map(str::trim).collect();
        if parts.len() == 4 {
            return (parts[..3].to_vec(), Some(parts[3]));
        }
        return (parts, None);
    }
    (s.split_whitespace().collect(), None)
}

fn parse_rgb(s: &str) -> Result<Hsla, String> {
    let (parts, alpha) = split_args(s);
    if parts.len() != 3 {
        return Err(format!("rgb() expects 3 channels, got {}", parts.len()));
    }
    let r = parse_rgb_channel(parts[0])?;
    let g = parse_rgb_channel(parts[1])?;
    let b = parse_rgb_channel(parts[2])?;
    let a = alpha.map(parse_alpha).transpose()?.unwrap_or(1.0);
    Ok(Hsla::from(Rgba { r, g, b, a }))
}

fn parse_rgb_channel(s: &str) -> Result<f32, String> {
    if let Some(pct) = s.strip_suffix('%') {
        let v: f32 = pct
            .trim()
            .parse()
            .map_err(|e: std::num::ParseFloatError| format!("bad rgb percentage {s:?}: {e}"))?;
        Ok((v / 100.0).clamp(0.0, 1.0))
    } else {
        let v: f32 = s
            .parse()
            .map_err(|e: std::num::ParseFloatError| format!("bad rgb channel {s:?}: {e}"))?;
        Ok((v / 255.0).clamp(0.0, 1.0))
    }
}

fn parse_alpha(s: &str) -> Result<f32, String> {
    let s = s.trim();
    if let Some(pct) = s.strip_suffix('%') {
        let v: f32 = pct
            .trim()
            .parse()
            .map_err(|e: std::num::ParseFloatError| format!("bad alpha percentage {s:?}: {e}"))?;
        Ok((v / 100.0).clamp(0.0, 1.0))
    } else {
        let v: f32 = s
            .parse()
            .map_err(|e: std::num::ParseFloatError| format!("bad alpha {s:?}: {e}"))?;
        Ok(v.clamp(0.0, 1.0))
    }
}

fn parse_hsl(s: &str) -> Result<Hsla, String> {
    let (parts, alpha) = split_args(s);
    if parts.len() != 3 {
        return Err(format!("hsl() expects 3 channels, got {}", parts.len()));
    }
    let h = parse_hue(parts[0])?;
    let sat = parse_percent(parts[1], "hsl saturation")?;
    let lit = parse_percent(parts[2], "hsl lightness")?;
    let a = alpha.map(parse_alpha).transpose()?.unwrap_or(1.0);
    Ok(Hsla {
        h: h.rem_euclid(360.0) / 360.0,
        s: sat.clamp(0.0, 1.0),
        l: lit.clamp(0.0, 1.0),
        a: a.clamp(0.0, 1.0),
    })
}

fn parse_percent(s: &str, label: &str) -> Result<f32, String> {
    let pct = s
        .strip_suffix('%')
        .ok_or_else(|| format!("{label} must be a percentage, got {s:?}"))?;
    let v: f32 = pct
        .parse()
        .map_err(|e: std::num::ParseFloatError| format!("bad {label} {s:?}: {e}"))?;
    Ok(v / 100.0)
}

fn parse_hue(s: &str) -> Result<f32, String> {
    let s = s.trim_end_matches("deg");
    s.parse::<f32>().map_err(|e| format!("bad hue {s:?}: {e}"))
}

fn parse_oklch(s: &str) -> Result<Hsla, String> {
    let (parts, alpha) = split_args(s);
    if parts.len() != 3 {
        return Err(format!("oklch() expects 3 channels, got {}", parts.len()));
    }
    let l = parse_oklch_lightness(parts[0])?;
    let c: f32 = parts[1]
        .parse()
        .map_err(|e: std::num::ParseFloatError| format!("bad oklch chroma {:?}: {e}", parts[1]))?;
    let h = parse_hue(parts[2])?;
    let a = alpha.map(parse_alpha).transpose()?.unwrap_or(1.0);
    Ok(oklch_to_hsla(l, c, h, a))
}

fn parse_oklch_lightness(s: &str) -> Result<f32, String> {
    if let Some(pct) = s.strip_suffix('%') {
        let v: f32 = pct
            .parse()
            .map_err(|e: std::num::ParseFloatError| format!("bad oklch lightness {s:?}: {e}"))?;
        Ok((v / 100.0).clamp(0.0, 1.0))
    } else {
        let v: f32 = s
            .parse()
            .map_err(|e: std::num::ParseFloatError| format!("bad oklch lightness {s:?}: {e}"))?;
        Ok(v.clamp(0.0, 1.0))
    }
}

/// OKLCH → sRGB [`Hsla`] via Björn Ottosson's OKLab → linear-sRGB matrix,
/// then the standard sRGB gamma encode.
#[allow(clippy::excessive_precision)]
fn oklch_to_hsla(l: f32, c: f32, h_deg: f32, a: f32) -> Hsla {
    let h = h_deg.to_radians();
    let ok_a = c * h.cos();
    let ok_b = c * h.sin();

    let l_ = l + 0.3963377774 * ok_a + 0.2158037573 * ok_b;
    let m_ = l - 0.1055613458 * ok_a - 0.0638541728 * ok_b;
    let s_ = l - 0.0894841775 * ok_a - 1.2914855480 * ok_b;
    let l3 = l_ * l_ * l_;
    let m3 = m_ * m_ * m_;
    let s3 = s_ * s_ * s_;

    let r_lin = 4.0767416621 * l3 - 3.3077115913 * m3 + 0.2309699292 * s3;
    let g_lin = -1.2684380046 * l3 + 2.6097574011 * m3 - 0.3413193965 * s3;
    let b_lin = -0.0041960863 * l3 - 0.7034186147 * m3 + 1.7076147010 * s3;

    Hsla::from(Rgba {
        r: linear_to_srgb(r_lin),
        g: linear_to_srgb(g_lin),
        b: linear_to_srgb(b_lin),
        a,
    })
}

fn linear_to_srgb(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    if x <= 0.0031308 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

/// serde bridge for plain `Hsla` fields - the 16 Base16 slots.
pub(crate) mod required {
    use super::{Deserialize, Deserializer, Hsla, Serialize, Serializer, parse};
    use serde::de::Error as _;

    pub(crate) fn deserialize<'de, D>(d: D) -> Result<Hsla, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(d)?;
        parse(&raw).map_err(D::Error::custom)
    }

    pub(crate) fn serialize<S>(v: &Hsla, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        v.serialize(s)
    }
}

/// serde bridge for `Option<Hsla>` refinement fields. Deserialization accepts
/// any of the four color forms above; serialization defers to gpui's stock
/// [`Hsla`] impl (emits `#rrggbbaa`) so round-trips stay canonical.
pub(crate) mod opt {
    use super::{Deserialize, Deserializer, Hsla, Serialize, Serializer, parse};
    use serde::de::Error as _;

    pub(crate) fn deserialize<'de, D>(d: D) -> Result<Option<Hsla>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Option::<String>::deserialize(d)?;
        raw.map(|s| parse(&s).map_err(D::Error::custom)).transpose()
    }

    pub(crate) fn serialize<S>(v: &Option<Hsla>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        v.serialize(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn hex_long() {
        let c = parse("#1d2021").unwrap();
        let rgba: Rgba = c.into();
        assert!(approx(rgba.r, 29.0 / 255.0, 1e-3));
        assert!(approx(rgba.g, 32.0 / 255.0, 1e-3));
        assert!(approx(rgba.b, 33.0 / 255.0, 1e-3));
        assert!(approx(rgba.a, 1.0, 1e-3));
    }

    #[test]
    fn hex_short_alpha() {
        let c = parse("#f0a8").unwrap();
        let rgba: Rgba = c.into();
        assert!(approx(rgba.r, 1.0, 1e-3));
        assert!(approx(rgba.g, 0.0, 1e-3));
        assert!(approx(rgba.b, 170.0 / 255.0, 1e-3));
        assert!(approx(rgba.a, 136.0 / 255.0, 1e-3));
    }

    /// Base16 YAML in the wild omits the leading `#` about as often as it
    /// includes it.
    #[test]
    fn hex_without_hash() {
        assert_eq!(parse("1e1e2e").unwrap(), parse("#1e1e2e").unwrap());
    }

    #[test]
    fn rgb_modern_and_legacy() {
        let a = parse("rgb(255 0 0)").unwrap();
        let b = parse("rgb(255, 0, 0)").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn rgb_slash_alpha() {
        let c = parse("rgb(255 0 0 / 0.5)").unwrap();
        let rgba: Rgba = c.into();
        assert!(approx(rgba.a, 0.5, 1e-3));
    }

    #[test]
    fn hsl_basic() {
        let c = parse("hsl(210 80% 50%)").unwrap();
        assert!(approx(c.h, 210.0 / 360.0, 1e-4));
        assert!(approx(c.s, 0.80, 1e-4));
        assert!(approx(c.l, 0.50, 1e-4));
    }

    #[test]
    fn oklch_parses() {
        let c = parse("oklch(0.18 0.004 70)").unwrap();
        assert!(c.l > 0.0 && c.l < 0.3);
    }

    #[test]
    fn rejects_nonsense() {
        assert!(parse("rebeccapurple").is_err());
        assert!(parse("oklch(none 0.1 45)").is_err());
        assert!(parse("#12345").is_err());
    }
}
