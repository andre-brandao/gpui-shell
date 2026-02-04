//! Theme module providing consistent styling across the application.
//!
//! This module defines the theme system and styling constants to ensure
//! a cohesive visual appearance throughout the bar, launcher, and panels.
//!
//! # Usage
//!
//! Access theme colors through the `ActiveTheme` trait:
//!
//! ```ignore
//! use ui::ActiveTheme;
//!
//! fn render(&mut self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
//!     let theme = cx.theme();
//!     div()
//!         .bg(theme.bg.primary)
//!         .text_color(theme.text.primary)
//!         .border_color(theme.border.default)
//! }
//! ```

use gpui::{App, Global, Hsla, Pixels, px, rgba};

mod colorize;
mod schemes;

pub use colorize::Colorize;
pub use schemes::{ThemeScheme, builtin_schemes};

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

    /// Replace the global theme with a new one.
    ///
    /// Call this to swap themes at runtime. The theme service will use this
    /// when loading Base16 schemes or switching themes.
    pub fn set(theme: Theme, cx: &mut App) {
        *cx.global_mut::<Theme>() = theme;
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
// Design Constants
// =============================================================================

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
