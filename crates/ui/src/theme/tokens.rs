//! Spacing, typography, and radius tokens.
//!
//! Every token is a fixed value except [`TextSize`], which is expressed in
//! `rems` so the whole UI scales with the user's configured base font size
//! (`Theme::font_size`, installed on each window via
//! [`gpui::Window::set_rem_size`]). That keeps the shell's existing
//! "one knob scales all text" behaviour while giving components a
//! semantic size vocabulary instead of raw pixel lookups.

use gpui::{Pixels, Rems, px, rems};

/// Semantic spacing step used for gaps, padding, and margins.
///
/// Fixed pixels on purpose: spacing should not grow with the font size, or
/// a bar configured for large text turns into a wall of padding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Spacing {
    /// 0px
    None,
    /// 2px
    XXSmall,
    /// 4px
    XSmall,
    /// 6px
    Small,
    /// 8px
    Medium,
    /// 12px
    Large,
    /// 16px
    XLarge,
    /// 20px
    XXLarge,
    /// 24px
    XXXLarge,
}

impl Spacing {
    pub const fn pixels(self) -> Pixels {
        px(self.value())
    }

    /// The raw pixel count.
    ///
    /// `Pixels` keeps its inner `f32` private, so call sites doing
    /// arithmetic - or defining a `const` - need the number itself.
    pub const fn value(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::XXSmall => 2.0,
            Self::XSmall => 4.0,
            Self::Small => 6.0,
            Self::Medium => 8.0,
            Self::Large => 12.0,
            Self::XLarge => 16.0,
            Self::XXLarge => 20.0,
            Self::XXXLarge => 24.0,
        }
    }
}

/// Semantic text size, expressed as a ratio of the configured base font
/// size.
///
/// [`TextSize::Default`] is exactly one rem, so it resolves to whatever
/// `Theme::font_size` the user set. The other ratios reproduce the shell's
/// previous `FontSizes` scale (xs/sm/base/md/lg/xl) one-for-one, so
/// switching to these tokens is not a visual change.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextSize {
    /// 0.77 rem (~10px at a 13px base)
    XSmall,
    /// 0.85 rem (~11px at a 13px base)
    Small,
    /// 1 rem - the configured base font size
    #[default]
    Default,
    /// 1.08 rem (~14px at a 13px base)
    Medium,
    /// 1.23 rem (~16px at a 13px base)
    Large,
    /// 1.38 rem (~18px at a 13px base)
    XLarge,
}

impl TextSize {
    pub fn rems(self) -> Rems {
        rems(self.ratio())
    }

    /// This size's multiplier over the base font size.
    pub const fn ratio(self) -> f32 {
        match self {
            Self::XSmall => 0.77,
            Self::Small => 0.85,
            Self::Default => 1.0,
            Self::Medium => 1.08,
            Self::Large => 1.23,
            Self::XLarge => 1.38,
        }
    }

    /// Resolve to absolute pixels against an explicit base font size.
    ///
    /// Prefer [`TextSize::rems`] - it lets gpui do the resolution against
    /// the window's rem size. This exists for the few places that need a
    /// concrete `Pixels` outside of a styled element.
    pub fn pixels(self, base: Pixels) -> Pixels {
        base * self.ratio()
    }
}

/// Semantic icon size.
///
/// Fixed pixels, matching the shell's previous `icon_size::{SM,MD,LG,XL}`
/// constants (12/14/16/18) with a 10px indicator step below them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconSize {
    /// 10px - status dots and other decorations.
    Indicator,
    /// 12px
    XSmall,
    /// 14px
    Small,
    /// 16px
    #[default]
    Medium,
    /// 18px
    Large,
    /// 20px
    XLarge,
}

impl IconSize {
    pub const fn pixels(self) -> Pixels {
        px(self.value())
    }

    /// The raw pixel count.
    ///
    /// `Pixels` keeps its inner `f32` private, so call sites doing
    /// arithmetic - or defining a `const` - need the number itself.
    pub const fn value(self) -> f32 {
        match self {
            Self::Indicator => 10.0,
            Self::XSmall => 12.0,
            Self::Small => 14.0,
            Self::Medium => 16.0,
            Self::Large => 18.0,
            Self::XLarge => 20.0,
        }
    }
}

/// Semantic border radius.
///
/// The ladder covers both the shell's previous `radius::{SM,MD,LG}`
/// constants (4/6/8) and the smaller 2px step used by dense controls.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Radius {
    /// 0px
    None,
    /// 2px
    XSmall,
    /// 4px
    Small,
    /// 6px
    #[default]
    Medium,
    /// 8px
    Large,
    /// 12px
    XLarge,
    /// Fully rounded (pill).
    Full,
}

impl Radius {
    pub const fn pixels(self) -> Pixels {
        px(self.value())
    }

    /// The raw pixel count.
    ///
    /// `Pixels` keeps its inner `f32` private, so call sites doing
    /// arithmetic - or defining a `const` - need the number itself.
    pub const fn value(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::XSmall => 2.0,
            Self::Small => 4.0,
            Self::Medium => 6.0,
            Self::Large => 8.0,
            Self::XLarge => 12.0,
            Self::Full => 9999.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_is_monotonically_increasing() {
        let steps = [
            Spacing::None,
            Spacing::XXSmall,
            Spacing::XSmall,
            Spacing::Small,
            Spacing::Medium,
            Spacing::Large,
            Spacing::XLarge,
            Spacing::XXLarge,
            Spacing::XXXLarge,
        ];
        for window in steps.windows(2) {
            assert!(
                window[0].pixels() < window[1].pixels(),
                "{:?} ({:?}) should be less than {:?} ({:?})",
                window[0],
                window[0].pixels(),
                window[1],
                window[1].pixels(),
            );
        }
    }

    #[test]
    fn radius_is_monotonically_increasing() {
        let steps = [
            Radius::None,
            Radius::XSmall,
            Radius::Small,
            Radius::Medium,
            Radius::Large,
            Radius::XLarge,
            Radius::Full,
        ];
        for window in steps.windows(2) {
            assert!(
                window[0].pixels() < window[1].pixels(),
                "{:?} should be smaller than {:?}",
                window[0],
                window[1],
            );
        }
    }

    #[test]
    fn text_size_is_monotonically_increasing() {
        let steps = [
            TextSize::XSmall,
            TextSize::Small,
            TextSize::Default,
            TextSize::Medium,
            TextSize::Large,
            TextSize::XLarge,
        ];
        for window in steps.windows(2) {
            assert!(
                window[0].ratio() < window[1].ratio(),
                "{:?} should be smaller than {:?}",
                window[0],
                window[1],
            );
        }
    }

    #[test]
    fn text_size_default_is_exactly_the_base() {
        assert_eq!(TextSize::Default.ratio(), 1.0);
        assert_eq!(TextSize::Default.pixels(px(13.0)), px(13.0));
    }

    /// The ratios must keep reproducing the scale the shell shipped before
    /// (`FontSizes::new(13.0)`), or every module silently reflows.
    #[test]
    fn text_size_reproduces_the_previous_font_scale() {
        let base = px(13.0);
        for (size, expected) in [
            (TextSize::XSmall, 10.01),
            (TextSize::Small, 11.05),
            (TextSize::Default, 13.0),
            (TextSize::Medium, 14.04),
            (TextSize::Large, 15.99),
            (TextSize::XLarge, 17.94),
        ] {
            let got: f32 = size.pixels(base).into();
            assert!(
                (got - expected).abs() < 0.01,
                "{size:?}: got {got}, expected {expected}"
            );
        }
    }
}
