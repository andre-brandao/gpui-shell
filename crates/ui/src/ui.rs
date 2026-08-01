//! Shared UI layer for the shell: the base16 theme system and the component
//! set built on top of it.

pub mod components;
pub mod patterns;
pub mod styles;
mod theme;
pub mod traits;

pub use components::*;
// `patterns` is intentionally not re-exported: composite, shell-specific
// surfaces stay behind `ui::patterns::` so they never blend into the
// primitive set. See the module docs.
pub use styles::ElevationIndex;
pub use theme::{
    ActiveTheme, Appearance, Base16Palette, Color, IconSize, Radius, Spacing, StatusColors,
    StatusColorsRefinement, StoredTheme, TextSize, Theme, ThemeColors, ThemeColorsRefinement,
    ThemeScheme, builtin_schemes,
};
pub use traits::*;

/// Register the default keybindings for components that need them -
/// [`TextField`] (navigation, selection, clipboard, submit) and [`Menu`]
/// (arrow navigation, Enter / Escape).
///
/// Call once at startup, after the theme is installed. Idempotent.
pub fn init(cx: &mut gpui::App) {
    use components::{menu, text_field};

    if cx.has_global::<UiInitialized>() {
        return;
    }
    text_field::bind_text_field_keys(cx);
    menu::bind_menu_keys(cx);
    cx.set_global(UiInitialized);
}

struct UiInitialized;
impl gpui::Global for UiInitialized {}
