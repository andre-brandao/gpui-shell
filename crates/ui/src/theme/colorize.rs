//! Color manipulation utilities.
//!
//! This module provides the `Colorize` trait which extends `Hsla` with
//! common color manipulation operations.

use anyhow::{anyhow, Result};
use gpui::Hsla;

/// Trait for color manipulation operations.
///
/// Provides methods for adjusting opacity, lightness, and mixing colors.
pub trait Colorize: Sized {
    /// Returns a new color with the given opacity.
    ///
    /// The opacity is a value between 0.0 and 1.0, where 0.0 is fully
    /// transparent and 1.0 is fully opaque.
    fn opacity(&self, opacity: f32) -> Self;

    /// Returns a new color with the alpha channel set to the given value.
    fn alpha(&self, alpha: f32) -> Self;

    /// Return inverted color.
    fn invert(&self) -> Self;

    /// Return inverted lightness.
    fn invert_l(&self) -> Self;

    /// Return a new color with the lightness increased by the given factor.
    ///
    /// Factor range: 0.0 .. 1.0
    fn lighten(&self, amount: f32) -> Self;

    /// Return a new color with the darkness increased by the given factor.
    ///
    /// Factor range: 0.0 .. 1.0
    fn darken(&self, amount: f32) -> Self;

    /// Mix two colors together.
    ///
    /// The `factor` is a value between 0.0 and 1.0 representing the weight
    /// of the first color.
    fn mix(&self, other: Self, factor: f32) -> Self;

    /// Blend this color over another (alpha compositing).
    fn blend(&self, other: Self) -> Self;

    /// Change the hue of the color.
    ///
    /// Hue range: 0.0 .. 1.0
    fn hue(&self, hue: f32) -> Self;

    /// Change the saturation of the color.
    ///
    /// Saturation range: 0.0 .. 1.0
    fn saturation(&self, saturation: f32) -> Self;

    /// Change the lightness of the color.
    ///
    /// Lightness range: 0.0 .. 1.0
    fn lightness(&self, lightness: f32) -> Self;

    /// Convert the color to a hex string (e.g., "#F8FAFC").
    fn to_hex(&self) -> String;

    /// Parse a hex string to a color.
    fn parse_hex(hex: &str) -> Result<Self>;
}

impl Colorize for Hsla {
    fn opacity(&self, factor: f32) -> Self {
        Self {
            a: self.a * factor.clamp(0.0, 1.0),
            ..*self
        }
    }

    fn alpha(&self, alpha: f32) -> Self {
        Self {
            a: alpha.clamp(0.0, 1.0),
            ..*self
        }
    }

    fn invert(&self) -> Self {
        Self {
            h: 1.0 - self.h,
            s: 1.0 - self.s,
            l: 1.0 - self.l,
            a: self.a,
        }
    }

    fn invert_l(&self) -> Self {
        Self {
            l: 1.0 - self.l,
            ..*self
        }
    }

    fn lighten(&self, factor: f32) -> Self {
        let l = self.l * (1.0 + factor.clamp(0.0, 1.0));
        Self { l, ..*self }
    }

    fn darken(&self, factor: f32) -> Self {
        let l = self.l * (1.0 - factor.clamp(0.0, 1.0));
        Self { l, ..*self }
    }

    fn mix(&self, other: Self, factor: f32) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        let inv = 1.0 - factor;

        #[inline]
        fn lerp_hue(a: f32, b: f32, t: f32) -> f32 {
            let diff = (b - a + 180.0).rem_euclid(360.0) - 180.0;
            (a + diff * t).rem_euclid(360.0)
        }

        Self {
            h: lerp_hue(self.h * 360.0, other.h * 360.0, factor) / 360.0,
            s: self.s * factor + other.s * inv,
            l: self.l * factor + other.l * inv,
            a: self.a * factor + other.a * inv,
        }
    }

    fn blend(&self, background: Self) -> Self {
        let alpha = self.a + background.a * (1.0 - self.a);
        if alpha == 0.0 {
            return Self {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.0,
            };
        }

        // Convert to RGB for blending, then back to HSL
        let fg_rgb = self.to_rgb();
        let bg_rgb = background.to_rgb();

        let r = (fg_rgb.r * self.a + bg_rgb.r * background.a * (1.0 - self.a)) / alpha;
        let g = (fg_rgb.g * self.a + bg_rgb.g * background.a * (1.0 - self.a)) / alpha;
        let b = (fg_rgb.b * self.a + bg_rgb.b * background.a * (1.0 - self.a)) / alpha;

        let rgba = gpui::Rgba { r, g, b, a: alpha };
        rgba.into()
    }

    fn hue(&self, hue: f32) -> Self {
        Self {
            h: hue.clamp(0.0, 1.0),
            ..*self
        }
    }

    fn saturation(&self, saturation: f32) -> Self {
        Self {
            s: saturation.clamp(0.0, 1.0),
            ..*self
        }
    }

    fn lightness(&self, lightness: f32) -> Self {
        Self {
            l: lightness.clamp(0.0, 1.0),
            ..*self
        }
    }

    fn to_hex(&self) -> String {
        let rgb = self.to_rgb();

        if self.a < 1.0 {
            format!(
                "#{:02X}{:02X}{:02X}{:02X}",
                (rgb.r * 255.0) as u32,
                (rgb.g * 255.0) as u32,
                (rgb.b * 255.0) as u32,
                (self.a * 255.0) as u32
            )
        } else {
            format!(
                "#{:02X}{:02X}{:02X}",
                (rgb.r * 255.0) as u32,
                (rgb.g * 255.0) as u32,
                (rgb.b * 255.0) as u32
            )
        }
    }

    fn parse_hex(hex: &str) -> Result<Self> {
        let hex = hex.trim_start_matches('#');
        let len = hex.len();

        if len != 6 && len != 8 {
            return Err(anyhow!("invalid hex color: expected 6 or 8 characters"));
        }

        let r = u8::from_str_radix(&hex[0..2], 16)? as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16)? as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16)? as f32 / 255.0;
        let a = if len == 8 {
            u8::from_str_radix(&hex[6..8], 16)? as f32 / 255.0
        } else {
            1.0
        };

        let rgba = gpui::Rgba { r, g, b, a };
        Ok(rgba.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::hsla;

    #[test]
    fn test_opacity() {
        let color = hsla(0.5, 0.5, 0.5, 1.0);
        let faded = color.opacity(0.5);
        assert!((faded.a - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_lighten_darken() {
        let color = hsla(0.5, 0.5, 0.5, 1.0);

        let lighter = color.lighten(0.2);
        assert!(lighter.l > color.l);

        let darker = color.darken(0.2);
        assert!(darker.l < color.l);
    }

    #[test]
    fn test_hex_roundtrip() {
        let original = hsla(0.0, 1.0, 0.5, 1.0); // Pure red
        let hex = original.to_hex();
        let parsed = Hsla::parse_hex(&hex).unwrap();

        // Allow some tolerance due to color space conversion
        assert!((original.l - parsed.l).abs() < 0.02);
    }

    #[test]
    fn test_parse_hex() {
        let color = Hsla::parse_hex("#FF0000").unwrap();
        let rgb = color.to_rgb();
        assert!((rgb.r - 1.0).abs() < 0.01);
        assert!(rgb.g < 0.01);
        assert!(rgb.b < 0.01);
    }
}
