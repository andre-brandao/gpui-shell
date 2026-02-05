//! Launcher view trait and shared utilities.
//!
//! This module defines the interface for launcher views and provides
//! helper functions for rendering common UI patterns.

use gpui::{AnyElement, App, FontWeight, MouseButton, div, prelude::*, px};
use services::Services;
use ui::{ActiveTheme, font_size, radius, spacing};

/// Height of each list item in pixels.
pub const LIST_ITEM_HEIGHT: f32 = 48.0;

/// Special characters that trigger view matching.
/// When a query starts with one of these, we look for a matching view.
pub const SPECIAL_CHARS: &[char] = &['@', '$', '!', '?', ';', '~', '#', ':'];

/// Input event passed to views for handling.
#[derive(Clone, Debug)]
pub enum ViewInput {
    /// Character typed.
    Char(String),
    /// Backspace pressed.
    Backspace,
    /// Up arrow pressed.
    Up,
    /// Down arrow pressed.
    Down,
    /// Page up pressed.
    PageUp,
    /// Page down pressed.
    PageDown,
    /// Enter pressed.
    Enter,
}

/// Result of handling input.
#[allow(dead_code)]
pub enum InputResult {
    /// Input was handled, optionally update the query and/or close.
    Handled {
        /// New query value (view-local part, without prefix).
        query: String,
        /// Whether to close the launcher.
        close: bool,
    },
    /// Input was not handled, use default behavior.
    Unhandled,
}

/// Context passed to views for rendering and actions.
pub struct ViewContext<'a> {
    pub services: &'a Services,
    pub query: &'a str,
    pub selected_index: usize,
}

/// A launcher view that provides custom rendering and input handling.
///
/// Views are responsible for:
/// - Declaring their prefix pattern (e.g., "@", "$", "!", ";ws")
/// - Rendering their content
/// - Handling selection and input
/// - Executing their own actions directly
pub trait LauncherView: Send + Sync {
    /// The prefix pattern that activates this view.
    ///
    /// Examples:
    /// - "@" for apps
    /// - "$" for shell commands
    /// - "!" for web search
    /// - "?" for help
    /// - ";ws" for workspaces
    fn prefix(&self) -> &'static str;

    /// Display name for the view.
    fn name(&self) -> &'static str;

    /// Icon for the view (Nerd font).
    fn icon(&self) -> &'static str;

    /// Description shown in help.
    fn description(&self) -> &'static str;

    /// Whether this view is the default when no prefix is given.
    fn is_default(&self) -> bool {
        false
    }

    /// Whether this view should appear in the help menu.
    fn show_in_help(&self) -> bool {
        true
    }

    /// Render the view content. Returns the element and number of selectable items.
    fn render(&self, vx: &ViewContext, cx: &App) -> (AnyElement, usize);

    /// Handle input. Return InputResult::Handled to consume the input.
    fn handle_input(&self, _input: &ViewInput, _vx: &ViewContext, _cx: &mut App) -> InputResult {
        InputResult::Unhandled
    }

    /// Handle item selection (Enter pressed or clicked).
    /// Return true to close the launcher.
    fn on_select(&self, _index: usize, _vx: &ViewContext, _cx: &mut App) -> bool {
        false
    }

    /// Return action hints to display in the footer bar.
    /// Each tuple is (action_name, keybinding).
    fn footer_actions(&self, _vx: &ViewContext) -> Vec<(&'static str, &'static str)> {
        vec![("Open", "Enter"), ("Close", "Esc")]
    }
}

/// Check if a character is a special prefix character.
pub fn is_special_char(c: char) -> bool {
    SPECIAL_CHARS.contains(&c)
}

/// Helper to render a standard list item.
pub fn render_list_item(
    id: impl Into<String>,
    icon: &str,
    title: &str,
    subtitle: Option<&str>,
    is_selected: bool,
    on_click: impl Fn(&mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();

    // Pre-compute colors for closures
    let accent_selection = theme.accent.selection;
    let interactive_hover = theme.interactive.hover;
    let interactive_default = theme.interactive.default;
    let text_primary = theme.text.primary;
    let text_disabled = theme.text.disabled;

    div()
        .id(id.into())
        .w_full()
        .h(px(LIST_ITEM_HEIGHT))
        .mx(px(spacing::SM))
        .px(px(spacing::SM))
        .rounded(px(radius::SM))
        .cursor_pointer()
        .flex()
        .items_center()
        .when(is_selected, move |el| el.bg(accent_selection))
        .when(!is_selected, move |el| {
            el.hover(move |s| s.bg(interactive_hover))
        })
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            on_click(cx);
        })
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(spacing::SM + 2.0))
                .child(
                    div()
                        .w(px(28.))
                        .h(px(28.))
                        .rounded(px(radius::SM))
                        .bg(interactive_default)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(font_size::MD))
                        .child(icon.to_string()),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .child(
                            div()
                                .text_size(px(font_size::BASE))
                                .text_color(text_primary)
                                .font_weight(FontWeight::MEDIUM)
                                .child(title.to_string()),
                        )
                        .when_some(subtitle, |el, sub| {
                            el.child(
                                div()
                                    .text_size(px(font_size::SM))
                                    .text_color(text_disabled)
                                    .child(sub.to_string()),
                            )
                        }),
                ),
        )
        .into_any_element()
}
