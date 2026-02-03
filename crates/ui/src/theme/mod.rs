//! Theme module providing consistent styling across the application.
//!
//! This module defines color constants, styling helpers, and a global theme
//! system to ensure a cohesive visual appearance throughout the bar, launcher,
//! and panels.
//!
//! # Usage
//!
//! Colors can be accessed either via module functions or through the theme trait:
//!
//! ```ignore
//! // Module-based access (existing pattern)
//! use ui::{bg, text, accent};
//! div().bg(bg::primary()).text_color(text::primary())
//!
//! // Trait-based access (new pattern)
//! use ui::ActiveTheme;
//! div().bg(cx.theme().bg.primary).text_color(cx.theme().text.primary)
//! ```

use gpui::{App, Global, Hsla, Pixels, px, rgba};

mod colorize;

pub use colorize::Colorize;

// =============================================================================
// Theme Struct and Trait
// =============================================================================

/// The global theme configuration.
///
/// This struct holds all theme values and is stored as a GPUI global.
/// Access it via the `ActiveTheme` trait on `App`.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Background colors
    pub bg: BgColors,
    /// Text/foreground colors
    pub text: TextColors,
    /// Border colors
    pub border: BorderColors,
    /// Accent/brand colors
    pub accent: AccentColors,
    /// Status indicator colors
    pub status: StatusColors,
    /// Interactive element colors
    pub interactive: InteractiveColors,

    /// General border radius
    pub radius: Pixels,
    /// Large border radius (cards, panels)
    pub radius_lg: Pixels,
    /// Fully transparent color
    pub transparent: Hsla,
}

impl Global for Theme {}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: BgColors::default(),
            text: TextColors::default(),
            border: BorderColors::default(),
            accent: AccentColors::default(),
            status: StatusColors::default(),
            interactive: InteractiveColors::default(),
            radius: px(6.0),
            radius_lg: px(8.0),
            transparent: Hsla::transparent_black(),
        }
    }
}

impl Theme {

    /// Initialize the global theme.
    ///
    /// Call this once at application startup.
    pub fn init(cx: &mut App) {
        cx.set_global(Theme::default());
    }

    /// Get a reference to the global theme.
    #[inline(always)]
    pub fn global(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }

    /// Get a mutable reference to the global theme.
    #[inline(always)]
    pub fn global_mut(cx: &mut App) -> &mut Theme {
        cx.global_mut::<Theme>()
    }
}

/// Trait for accessing the active theme.
///
/// This is implemented on `App` to provide convenient access to theme values.
pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    #[inline(always)]
    fn theme(&self) -> &Theme {
        Theme::global(self)
    }
}

// =============================================================================
// Color Groups
// =============================================================================

/// Background colors.
#[derive(Debug, Clone, Copy)]
pub struct BgColors {
    /// Primary background (darkest) - main containers
    pub primary: Hsla,
    /// Secondary background - cards/sections
    pub secondary: Hsla,
    /// Tertiary background - inputs, hover states
    pub tertiary: Hsla,
    /// Elevated background - dropdowns, tooltips
    pub elevated: Hsla,
}

impl Default for BgColors {
    fn default() -> Self {
        Self {
            primary: rgba(0x1e1e1eff).into(),
            secondary: rgba(0x252526ff).into(),
            tertiary: rgba(0x2d2d2dff).into(),
            elevated: rgba(0x333333ff).into(),
        }
    }
}

/// Text/foreground colors.
#[derive(Debug, Clone, Copy)]
pub struct TextColors {
    /// Primary text (brightest)
    pub primary: Hsla,
    /// Secondary text (slightly muted)
    pub secondary: Hsla,
    /// Muted text (labels, hints)
    pub muted: Hsla,
    /// Disabled text
    pub disabled: Hsla,
    /// Placeholder text
    pub placeholder: Hsla,
}

impl Default for TextColors {
    fn default() -> Self {
        Self {
            primary: rgba(0xffffffee).into(),
            secondary: rgba(0xccccccff).into(),
            muted: rgba(0x888888ff).into(),
            disabled: rgba(0x6e6e6eff).into(),
            placeholder: rgba(0x6e6e6eff).into(),
        }
    }
}

/// Border colors.
#[derive(Debug, Clone, Copy)]
pub struct BorderColors {
    /// Default border color
    pub default: Hsla,
    /// Subtle border (less visible)
    pub subtle: Hsla,
    /// Focused/active border
    pub focused: Hsla,
}

impl Default for BorderColors {
    fn default() -> Self {
        Self {
            default: rgba(0x3c3c3cff).into(),
            subtle: rgba(0x2d2d2dff).into(),
            focused: rgba(0x007accff).into(),
        }
    }
}

/// Accent/brand colors.
#[derive(Debug, Clone, Copy)]
pub struct AccentColors {
    /// Primary accent (Zed blue)
    pub primary: Hsla,
    /// Selection background
    pub selection: Hsla,
    /// Hover accent
    pub hover: Hsla,
}

impl Default for AccentColors {
    fn default() -> Self {
        Self {
            primary: rgba(0x007accff).into(),
            selection: rgba(0x094771ff).into(),
            hover: rgba(0x1177bbff).into(),
        }
    }
}

/// Status indicator colors.
#[derive(Debug, Clone, Copy)]
pub struct StatusColors {
    /// Success/good state
    pub success: Hsla,
    /// Warning state
    pub warning: Hsla,
    /// Error/critical state
    pub error: Hsla,
    /// Info state
    pub info: Hsla,
}

impl Default for StatusColors {
    fn default() -> Self {
        Self {
            success: rgba(0x4ade80ff).into(),
            warning: rgba(0xfbbf24ff).into(),
            error: rgba(0xf87171ff).into(),
            info: rgba(0x60a5faff).into(),
        }
    }
}

impl StatusColors {

    /// Get color based on percentage value (for progress bars, usage indicators).
    pub fn from_percentage(&self, value: u32) -> Hsla {
        if value >= 90 {
            self.error
        } else if value >= 70 {
            self.warning
        } else {
            self.success
        }
    }

    /// Get color based on temperature.
    pub fn from_temperature(&self, temp: i32) -> Hsla {
        if temp >= 85 {
            self.error
        } else if temp >= 70 {
            self.warning
        } else {
            self.success
        }
    }
}

/// Interactive element colors (buttons, toggles).
#[derive(Debug, Clone, Copy)]
pub struct InteractiveColors {
    /// Default state background
    pub default: Hsla,
    /// Hover state background
    pub hover: Hsla,
    /// Active/pressed state
    pub active: Hsla,
    /// Toggle on state
    pub toggle_on: Hsla,
    /// Toggle on hover
    pub toggle_on_hover: Hsla,
}

impl Default for InteractiveColors {
    fn default() -> Self {
        Self {
            default: rgba(0x3b3b3bff).into(),
            hover: rgba(0x454545ff).into(),
            active: rgba(0x505050ff).into(),
            toggle_on: rgba(0x007accff).into(),
            toggle_on_hover: rgba(0x1177bbff).into(),
        }
    }
}

// =============================================================================
// Legacy Module-Based Access (kept for backwards compatibility)
// =============================================================================

/// Core background colors
pub mod bg {
    use super::*;

    pub const PRIMARY: u32 = 0x1e1e1eff;
    pub const SECONDARY: u32 = 0x252526ff;
    pub const TERTIARY: u32 = 0x2d2d2dff;
    pub const ELEVATED: u32 = 0x333333ff;

    pub fn primary() -> Hsla {
        rgba(PRIMARY).into()
    }

    pub fn secondary() -> Hsla {
        rgba(SECONDARY).into()
    }

    pub fn tertiary() -> Hsla {
        rgba(TERTIARY).into()
    }

    pub fn elevated() -> Hsla {
        rgba(ELEVATED).into()
    }
}

/// Border colors
pub mod border {
    use super::*;

    pub const DEFAULT: u32 = 0x3c3c3cff;
    pub const SUBTLE: u32 = 0x2d2d2dff;
    pub const FOCUSED: u32 = 0x007accff;

    pub fn default() -> Hsla {
        rgba(DEFAULT).into()
    }

    pub fn subtle() -> Hsla {
        rgba(SUBTLE).into()
    }

    pub fn focused() -> Hsla {
        rgba(FOCUSED).into()
    }
}

/// Text colors
pub mod text {
    use super::*;

    pub const PRIMARY: u32 = 0xffffffee;
    pub const SECONDARY: u32 = 0xccccccff;
    pub const MUTED: u32 = 0x888888ff;
    pub const DISABLED: u32 = 0x6e6e6eff;
    pub const PLACEHOLDER: u32 = 0x6e6e6eff;

    pub fn primary() -> Hsla {
        rgba(PRIMARY).into()
    }

    pub fn secondary() -> Hsla {
        rgba(SECONDARY).into()
    }

    pub fn muted() -> Hsla {
        rgba(MUTED).into()
    }

    pub fn disabled() -> Hsla {
        rgba(DISABLED).into()
    }

    pub fn placeholder() -> Hsla {
        rgba(PLACEHOLDER).into()
    }
}

/// Accent/brand colors
pub mod accent {
    use super::*;

    pub const PRIMARY: u32 = 0x007accff;
    pub const SELECTION: u32 = 0x094771ff;
    pub const HOVER: u32 = 0x1177bbff;

    pub fn primary() -> Hsla {
        rgba(PRIMARY).into()
    }

    pub fn selection() -> Hsla {
        rgba(SELECTION).into()
    }

    pub fn hover() -> Hsla {
        rgba(HOVER).into()
    }
}

/// Status colors for indicators
pub mod status {
    use super::*;

    pub const SUCCESS: u32 = 0x4ade80ff;
    pub const WARNING: u32 = 0xfbbf24ff;
    pub const ERROR: u32 = 0xf87171ff;
    pub const INFO: u32 = 0x60a5faff;

    pub fn success() -> Hsla {
        rgba(SUCCESS).into()
    }

    pub fn warning() -> Hsla {
        rgba(WARNING).into()
    }

    pub fn error() -> Hsla {
        rgba(ERROR).into()
    }

    pub fn info() -> Hsla {
        rgba(INFO).into()
    }

    /// Get color based on percentage value (for progress bars, usage indicators)
    pub fn from_percentage(value: u32) -> Hsla {
        if value >= 90 {
            error()
        } else if value >= 70 {
            warning()
        } else {
            success()
        }
    }

    /// Get color based on temperature
    pub fn from_temperature(temp: i32) -> Hsla {
        if temp >= 85 {
            error()
        } else if temp >= 70 {
            warning()
        } else {
            success()
        }
    }
}

/// Interactive element colors (buttons, toggles)
pub mod interactive {
    use super::*;

    pub const DEFAULT: u32 = 0x3b3b3bff;
    pub const HOVER: u32 = 0x454545ff;
    pub const ACTIVE: u32 = 0x505050ff;
    pub const TOGGLE_ON: u32 = 0x007accff;
    pub const TOGGLE_ON_HOVER: u32 = 0x1177bbff;

    pub fn default() -> Hsla {
        rgba(DEFAULT).into()
    }

    pub fn hover() -> Hsla {
        rgba(HOVER).into()
    }

    pub fn active() -> Hsla {
        rgba(ACTIVE).into()
    }

    pub fn toggle_on() -> Hsla {
        rgba(TOGGLE_ON).into()
    }

    pub fn toggle_on_hover() -> Hsla {
        rgba(TOGGLE_ON_HOVER).into()
    }
}

/// Spacing constants (in pixels)
pub mod spacing {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
}

/// Border radius constants (in pixels)
pub mod radius {
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 6.0;
    pub const LG: f32 = 8.0;
    pub const XL: f32 = 12.0;
}

/// Font sizes (in pixels)
pub mod font_size {
    pub const XS: f32 = 10.0;
    pub const SM: f32 = 11.0;
    pub const BASE: f32 = 13.0;
    pub const MD: f32 = 14.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 18.0;
}

/// Icon sizes (in pixels)
pub mod icon_size {
    pub const SM: f32 = 12.0;
    pub const MD: f32 = 14.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 18.0;
}
