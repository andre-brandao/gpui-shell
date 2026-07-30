//! Semantic color tokens used by every component.

use gpui::Hsla;

/// A semantic color reference that components use instead of raw [`Hsla`].
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Color {
    /// Default text / foreground color.
    #[default]
    Default,
    /// Muted / secondary foreground.
    Muted,
    /// Placeholder foreground (e.g. empty input hint).
    Placeholder,
    /// Disabled foreground.
    Disabled,
    /// Accent foreground (links, emphasis).
    Accent,
    /// Selected foreground - used for items in the selected/active state.
    Selected,
    /// Hint or suggestion text.
    Hint,
    /// Visually hidden / strongly de-emphasized foreground.
    Hidden,
    /// Intentionally ignored item.
    Ignored,
    /// Success / positive status.
    Success,
    /// Warning / caution status.
    Warning,
    /// Error / destructive status.
    Error,
    /// Informational status.
    Info,
    /// Raw color, bypassing the theme.
    Custom(Hsla),
}

impl Color {
    /// Resolve this semantic color to an [`Hsla`] using the given [`ThemeColors`].
    pub fn hsla(&self, colors: &ThemeColors) -> Hsla {
        match self {
            Self::Default => colors.text,
            Self::Muted => colors.text_muted,
            Self::Placeholder => colors.text_placeholder,
            Self::Disabled => colors.text_disabled,
            Self::Accent | Self::Selected => colors.text_accent,
            Self::Hint => colors.status.hint,
            Self::Hidden => colors.status.hidden,
            Self::Ignored => colors.status.ignored,
            Self::Success => colors.status.success,
            Self::Warning => colors.status.warning,
            Self::Error => colors.status.error,
            Self::Info => colors.status.info,
            Self::Custom(hsla) => *hsla,
        }
    }
}

/// Status color group: the diagnostic / informational flavors, each with a
/// base foreground variant plus a background and border for status surfaces
/// (inline banners, toast chrome, battery/temperature warnings).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusColors {
    pub info: Hsla,
    pub info_background: Hsla,
    pub info_border: Hsla,

    pub success: Hsla,
    pub success_background: Hsla,
    pub success_border: Hsla,

    pub warning: Hsla,
    pub warning_background: Hsla,
    pub warning_border: Hsla,

    pub error: Hsla,
    pub error_background: Hsla,
    pub error_border: Hsla,

    /// Hint or suggestion text.
    pub hint: Hsla,
    pub hint_background: Hsla,
    pub hint_border: Hsla,

    /// Strongly de-emphasized - present but should not draw the eye.
    pub hidden: Hsla,
    pub hidden_background: Hsla,
    pub hidden_border: Hsla,

    /// Items intentionally ignored.
    pub ignored: Hsla,
    pub ignored_background: Hsla,
    pub ignored_border: Hsla,
}

impl StatusColors {
    /// Pick a status color from a 0-100 usage percentage.
    pub fn from_percentage(&self, value: u32) -> Hsla {
        if value >= 90 {
            self.error
        } else if value >= 70 {
            self.warning
        } else {
            self.success
        }
    }

    /// Pick a status color from a temperature in degrees Celsius.
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

/// The semantic color palette powering every component.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeColors {
    // Surfaces
    /// Window / root background.
    pub background: Hsla,
    /// Grounded surface (bar, panel, sidebar).
    pub surface_background: Hsla,
    /// Elevated surface (popover, dropdown, card).
    pub elevated_surface_background: Hsla,

    // Borders
    /// Default border color.
    pub border: Hsla,
    /// Subtle border used for dividers between related content.
    pub border_variant: Hsla,
    /// Border for keyboard focus ring.
    pub border_focused: Hsla,
    /// Border for the active / selected state.
    pub border_selected: Hsla,
    /// Border for disabled elements.
    pub border_disabled: Hsla,
    /// A fully transparent border.
    pub border_transparent: Hsla,

    // Foreground text
    pub text: Hsla,
    pub text_muted: Hsla,
    pub text_placeholder: Hsla,
    pub text_disabled: Hsla,
    pub text_accent: Hsla,

    // Foreground icons
    pub icon: Hsla,
    pub icon_muted: Hsla,
    pub icon_disabled: Hsla,
    pub icon_accent: Hsla,

    // Filled (opaque) interactive element backgrounds
    pub element_background: Hsla,
    pub element_hover: Hsla,
    pub element_active: Hsla,
    pub element_selected: Hsla,
    pub element_disabled: Hsla,

    // Ghost (transparent) interactive element backgrounds
    /// Resting background for a ghost element.
    pub ghost_element_background: Hsla,
    pub ghost_element_hover: Hsla,
    pub ghost_element_active: Hsla,
    pub ghost_element_selected: Hsla,
    pub ghost_element_disabled: Hsla,

    // Status / semantic
    pub status: StatusColors,

    /// Primary accent color.
    pub accent: Hsla,
}
