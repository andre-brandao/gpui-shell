mod components;
mod theme;
mod traits;

// Re-export components
pub use components::{
    // Label
    Color,
    // List
    CursorPlacement,
    EmptyMessage,
    InputBuffer,
    Label,
    LabelCommon,
    // Layout
    LabelSide,
    LabelSize,
    List,
    ListItem,
    ListItemSpacing,
    ListSeparator,
    MaskedRenderParts,
    PlainRenderParts,
    Slider,
    SliderEvent,
    Switch,
    SwitchSize,
    render_input_line,
    render_masked_input_line,
};

// Layout helpers are used by this crate's own components; the app styles
// with plain gpui builders, so they stay crate-internal.
pub(crate) use components::{h_flex, v_flex};

// Re-export theme system
pub use theme::{
    // Color group types (for constructing themes)
    AccentColors,
    // Core theme types
    ActiveTheme,
    // Base16 conversion
    Base16Colors,
    BgColors,
    BorderColors,
    Colorize,
    // Font sizing
    FontSizes,
    InteractiveColors,
    StatusColors,
    TextColors,
    Theme,
    // Theme schemes
    ThemeScheme,
    builtin_schemes,
    // Design constants (non-color)
    icon_size,
    radius,
    spacing,
};
