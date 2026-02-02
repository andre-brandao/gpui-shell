mod components;
mod theme;
mod traits;

// Re-export components
pub use components::{Slider, SliderEvent, h_flex, v_flex};

// Re-export traits
pub use traits::styled_ext::StyledExt;

// Re-export theme submodules for convenient access
pub use theme::{
    accent, bg, border, font_size, icon_size, interactive, radius, spacing, status, text,
};
