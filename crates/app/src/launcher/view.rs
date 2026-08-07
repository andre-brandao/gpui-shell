//! Launcher view trait and shared utilities.
//!
//! This module defines the interface for launcher views and provides
//! helper functions for rendering common UI patterns.
//!
//! Views come in two flavours:
//!
//! - **List views** (e.g. apps, workspaces, help) — implement `render_item()` to render
//!   individual selectable rows. The launcher iterates `match_count()` times.
//!
//! - **Content views** (e.g. shell, web) — override `render_content()` to
//!   return a single element for their entire body. When this returns `Some`,
//!   the launcher skips the item loop.

use gpui::{AnyElement, App};
use ui::IconName;
use ui::patterns::footer_hints;

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
    fn prefix(&self) -> &str;

    /// Display name for the view.
    fn name(&self) -> &'static str;

    /// Icon for the view.
    fn icon(&self) -> IconName;

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

    /// Recompute the view's match list against the current query.
    ///
    /// The launcher calls this exactly once per frame, and again whenever
    /// it needs a fresh count outside of rendering. Everything after it -
    /// [`Self::match_count`], [`Self::render_item`], [`Self::on_select`] -
    /// must read the stored result rather than filtering again: those are
    /// called once *per row*, so filtering inside them is quadratic in the
    /// number of matches.
    ///
    /// Content views that have no list leave this as a no-op.
    fn update_matches(&mut self, _vx: &ViewContext, _cx: &App) {}

    /// How many selectable items the view currently has.
    ///
    /// Reads the result of the last [`Self::update_matches`].
    fn match_count(&self, vx: &ViewContext, cx: &App) -> usize;

    /// Render a single list item at `index`. `selected` is true if the
    /// launcher's selection cursor is on this item.
    ///
    /// Called only for the rows gpui actually needs, and reads the result
    /// of the last [`Self::update_matches`].
    fn render_item(&self, index: usize, selected: bool, vx: &ViewContext, cx: &App) -> AnyElement;

    /// Optional header rendered above the item list.
    fn render_header(&self, _vx: &ViewContext, _cx: &App) -> Option<AnyElement> {
        None
    }

    /// Optional section rendered below the item list.
    fn render_footer(&self, _vx: &ViewContext, _cx: &App) -> Option<AnyElement> {
        None
    }

    /// Full-content rendering for non-list views. When this returns `Some`,
    /// `render_item()` is not called and the returned element is used as
    /// the view body instead.
    fn render_content(&self, _vx: &ViewContext, _cx: &App) -> Option<AnyElement> {
        None
    }

    /// Handle input. Return InputResult::Handled to consume the input.
    fn handle_input(&self, _input: &ViewInput, _vx: &ViewContext, _cx: &mut App) -> InputResult {
        InputResult::Unhandled
    }

    /// Handle item selection (Enter pressed or clicked).
    /// Return true to close the launcher.
    fn on_select(&self, _index: usize, _vx: &ViewContext, _cx: &mut App) -> bool {
        false
    }

    /// Content for the footer's action side. Most views want key hints via
    /// [`footer_hints`], but any element goes.
    fn render_footer_bar(&self, _vx: &ViewContext, cx: &App) -> AnyElement {
        footer_hints(vec![("Open", "Enter"), ("Close", "Esc")], cx)
    }
}

/// Returns true if the query starts with a view prefix.
pub fn is_prefix(query: &str, prefix: &str) -> bool {
    query.starts_with(prefix)
}
