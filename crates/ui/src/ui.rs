//! Shared UI layer for the shell: the theme system and the component set
//! built on top of it.

mod components;
mod theme;
mod traits;

// Components
pub use components::{
    CursorPlacement, EmptyMessage, InputBuffer, Label, LabelCommon, LabelSide, List, ListItem,
    ListItemSpacing, ListSeparator, MaskedRenderParts, PlainRenderParts, Slider, SliderEvent,
    Switch, SwitchSize, h_flex, render_input_line, render_masked_input_line, v_flex,
};

// Traits
pub use traits::styled_ext::StyledExt;

// Theme
pub use theme::{
    ActiveTheme, Appearance, Base16Palette, Color, IconSize, Radius, Spacing, StatusColors,
    StatusColorsRefinement, StoredTheme, TextSize, Theme, ThemeColors, ThemeColorsRefinement,
    ThemeScheme, builtin_schemes,
};
