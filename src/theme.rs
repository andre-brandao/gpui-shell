//! Theme module providing consistent Zed-like styling across the application.
//!
//! This module defines color constants and styling helpers to ensure
//! a cohesive visual appearance throughout the bar, launcher, and panels.

use gpui::{Hsla, rgba};

/// Core background colors
pub mod bg {
    use super::*;

    /// Primary background (darkest) - used for main containers
    pub const PRIMARY: u32 = 0x1e1e1eff;
    /// Secondary background - used for cards/sections
    pub const SECONDARY: u32 = 0x252526ff;
    /// Tertiary background - used for inputs, hover states
    pub const TERTIARY: u32 = 0x2d2d2dff;
    /// Elevated background - used for dropdowns, tooltips
    pub const ELEVATED: u32 = 0x333333ff;

    /// Get primary background as Hsla
    pub fn primary() -> Hsla {
        rgba(PRIMARY).into()
    }

    /// Get secondary background as Hsla
    pub fn secondary() -> Hsla {
        rgba(SECONDARY).into()
    }

    /// Get tertiary background as Hsla
    pub fn tertiary() -> Hsla {
        rgba(TERTIARY).into()
    }

    /// Get elevated background as Hsla
    pub fn elevated() -> Hsla {
        rgba(ELEVATED).into()
    }
}

/// Border colors
pub mod border {
    use super::*;

    /// Default border color
    pub const DEFAULT: u32 = 0x3c3c3cff;
    /// Subtle border (less visible)
    pub const SUBTLE: u32 = 0x2d2d2dff;
    /// Focused/active border
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

    /// Primary text (brightest)
    pub const PRIMARY: u32 = 0xffffffee;
    /// Secondary text (slightly muted)
    pub const SECONDARY: u32 = 0xccccccff;
    /// Muted text (for labels, hints)
    pub const MUTED: u32 = 0x888888ff;
    /// Disabled text
    pub const DISABLED: u32 = 0x6e6e6eff;
    /// Placeholder text
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

    /// Primary accent (Zed blue)
    pub const PRIMARY: u32 = 0x007accff;
    /// Selection background (darker blue)
    pub const SELECTION: u32 = 0x094771ff;
    /// Hover accent
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

    /// Success/good state
    pub const SUCCESS: u32 = 0x4ade80ff;
    /// Warning state
    pub const WARNING: u32 = 0xfbbf24ff;
    /// Error/critical state
    pub const ERROR: u32 = 0xf87171ff;
    /// Info state
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

    /// Default state background
    pub const DEFAULT: u32 = 0x3b3b3bff;
    /// Hover state background
    pub const HOVER: u32 = 0x454545ff;
    /// Active/pressed state
    pub const ACTIVE: u32 = 0x505050ff;
    /// Toggle on state
    pub const TOGGLE_ON: u32 = 0x007accff;
    /// Toggle on hover
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
    /// Extra small spacing
    pub const XS: f32 = 4.0;
    /// Small spacing
    pub const SM: f32 = 8.0;
    /// Medium spacing (default)
    pub const MD: f32 = 12.0;
    /// Large spacing
    pub const LG: f32 = 16.0;
    /// Extra large spacing
    pub const XL: f32 = 24.0;
}

/// Border radius constants (in pixels)
pub mod radius {
    /// Small radius (for badges, small buttons)
    pub const SM: f32 = 4.0;
    /// Medium radius (for buttons, inputs)
    pub const MD: f32 = 6.0;
    /// Large radius (for cards, panels)
    pub const LG: f32 = 8.0;
    /// Extra large radius (for modals)
    pub const XL: f32 = 12.0;
}

/// Font sizes (in pixels)
pub mod font_size {
    /// Extra small (badges, labels)
    pub const XS: f32 = 10.0;
    /// Small (secondary text)
    pub const SM: f32 = 11.0;
    /// Base size
    pub const BASE: f32 = 13.0;
    /// Medium (slightly larger)
    pub const MD: f32 = 14.0;
    /// Large (headings)
    pub const LG: f32 = 16.0;
    /// Extra large (titles)
    pub const XL: f32 = 18.0;
}

/// Icon sizes (in pixels)
pub mod icon_size {
    /// Small icons (inline with text)
    pub const SM: f32 = 12.0;
    /// Medium icons (buttons, list items)
    pub const MD: f32 = 14.0;
    /// Large icons (prominent displays)
    pub const LG: f32 = 16.0;
    /// Extra large icons (headers)
    pub const XL: f32 = 18.0;
}
