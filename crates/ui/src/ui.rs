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
    h_flex,
    render_input_line,
    render_masked_input_line,
    v_flex,
};

// Re-export traits
pub use traits::styled_ext::StyledExt;

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
    font_size,
    icon_size,
    radius,
    spacing,
};
