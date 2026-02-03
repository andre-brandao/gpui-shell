mod components;
mod theme;
mod traits;

// Re-export components
pub use components::{LabelSide, Slider, SliderEvent, Switch, SwitchSize, h_flex, v_flex};

// Re-export traits
pub use traits::styled_ext::StyledExt;

// Re-export theme system
pub use theme::{ActiveTheme, Colorize, Theme};

// Re-export theme submodules for convenient access (legacy pattern)
pub use theme::{
    accent, bg, border, font_size, icon_size, interactive, radius, spacing, status, text,
};
