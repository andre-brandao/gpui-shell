mod components;
mod theme;
mod traits;

// Re-export components
pub use components::{LabelSide, Slider, SliderEvent, Switch, SwitchSize, h_flex, v_flex};

// Re-export traits
pub use traits::styled_ext::StyledExt;

// Re-export theme system
pub use theme::{
    // Core theme types
    ActiveTheme, Colorize, Theme,
    // Color group types (for constructing themes)
    AccentColors, BgColors, BorderColors, InteractiveColors, StatusColors, TextColors,
    // Base16 conversion
    Base16Colors,
    // Theme schemes
    ThemeScheme, builtin_schemes,
    // Design constants (non-color)
    font_size, icon_size, radius, spacing,
};
