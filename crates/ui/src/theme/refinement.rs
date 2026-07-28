//! Partial overrides on top of a base [`ThemeColors`].
//!
//! A [`ThemeColorsRefinement`] is "what a user's theme file is allowed to
//! set": every field mirrors [`ThemeColors`] but wrapped in [`Option`].
//! Fields the user omits stay `None` and fall through to whatever the
//! Base16 palette derived.
//!
//! Adding a new theme token therefore needs changes in exactly two places:
//! the base struct in [`colors`](super::colors) and the field list below.

use super::colors::{StatusColors, ThemeColors};

refineable! {
    /// A partial [`StatusColors`]. All fields optional.
    pub struct StatusColorsRefinement refines StatusColors {
        colors {
            info, info_background, info_border,
            success, success_background, success_border,
            warning, warning_background, warning_border,
            error, error_background, error_border,
            hint, hint_background, hint_border,
            hidden, hidden_background, hidden_border,
            ignored, ignored_background, ignored_border,
        }
    }
}

refineable! {
    /// A partial [`ThemeColors`]. All fields optional.
    pub struct ThemeColorsRefinement refines ThemeColors {
        colors {
            background, surface_background, elevated_surface_background,
            border, border_variant, border_focused, border_selected,
            border_disabled, border_transparent,
            text, text_muted, text_placeholder, text_disabled, text_accent,
            icon, icon_muted, icon_disabled, icon_accent,
            element_background, element_hover, element_active,
            element_selected, element_disabled,
            ghost_element_background, ghost_element_hover, ghost_element_active,
            ghost_element_selected, ghost_element_disabled,
            accent,
        }
        nested {
            status: StatusColorsRefinement,
        }
    }
}
