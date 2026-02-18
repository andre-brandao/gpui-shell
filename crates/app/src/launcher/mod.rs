//! Launcher module providing an application launcher overlay.
//!
//! The launcher provides a keyboard-driven interface for:
//! - Searching and launching applications (@ prefix, or default)
//! - Running shell commands ($ prefix)
//! - Web search with multiple providers (! prefix with shebangs)
//! - Switching workspaces (;ws prefix)
//! - Viewing help and available commands (? prefix)

pub mod config;
pub mod modules;
pub mod view;

use gpui::{
    App, Bounds, Context, FocusHandle, Focusable, Point, ScrollHandle, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, div,
    layer_shell::*, prelude::*, px,
};
use modules::{HelpView, all_views};
use ui::{ActiveTheme, InputBuffer, font_size, icon_size, radius, render_input_line, spacing};
use view::{InputResult, LauncherView, ViewContext, ViewInput, is_prefix};

use crate::config::Config;
use crate::keybinds::{
    Backspace, Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp, DeleteWordBack,
    PageDown, PageUp, SelectAll, SelectLeft, SelectRight, SelectWordLeft, SelectWordRight,
    WordLeft, WordRight,
};
use crate::state::{AppState, watch};

/// Number of items to jump when using Page Up/Down.
const ITEMS_PER_PAGE: usize = 7;

/// The main launcher struct.
pub struct Launcher {
    input: InputBuffer,
    selected_index: usize,
    focus_handle: FocusHandle,
    scroll_handle: ScrollHandle,
    views: Vec<Box<dyn LauncherView>>,
    help_view: HelpView,
}

impl Launcher {
    /// Create a new launcher with optional initial input.
    pub fn new(initial_input: Option<String>, cx: &mut Context<Self>) -> Self {
        let compositor = AppState::compositor(cx).clone();
        let sysinfo = AppState::sysinfo(cx).clone();
        let upower = AppState::upower(cx).clone();
        let focus_handle = cx.focus_handle();
        let scroll_handle = ScrollHandle::new();

        // Subscribe to service updates for reactive rendering
        watch(cx, compositor.subscribe(), |_, _, cx| {
            cx.notify();
        });

        watch(cx, sysinfo.subscribe(), |_, _, cx| {
            cx.notify();
        });

        watch(cx, upower.subscribe(), |_, _, cx| {
            cx.notify();
        });

        let launcher_config = Config::global(cx).launcher.clone();
        let views = all_views(&launcher_config);
        let help_view = HelpView::new(&launcher_config.modules.help, &views);

        Launcher {
            input: InputBuffer::new(initial_input.unwrap_or_default()),
            selected_index: 0,
            focus_handle,
            scroll_handle,
            views,
            help_view,
        }
    }

    /// Set the search query (used for IPC input).
    pub fn set_input(&mut self, input: String) {
        self.input.set_text(input);
        self.selected_index = 0;
        self.reset_scroll();
    }

    /// Reset scroll to top.
    fn reset_scroll(&self) {
        self.scroll_handle.set_offset(gpui::point(px(0.), px(0.)));
    }

    /// Ensure the selected item is scrolled into view.
    /// The header (if present) is the first child, so item indices are offset by 1.
    fn scroll_to_selected(&self, cx: &App) {
        let vx = self.view_context();
        let has_header = self.current_view().render_header(&vx, cx).is_some();
        let child_index = self.selected_index + if has_header { 1 } else { 0 };
        self.scroll_handle.scroll_to_item(child_index);
    }

    /// Parse the search query to find which view should handle it.
    /// Returns (matched_view_or_none, search_term_for_view).
    fn parse_query(&self) -> (Option<&dyn LauncherView>, &str) {
        let query = self.input.text().trim();

        if query.is_empty() {
            return (self.default_view(), "");
        }

        // Check if query starts with any view's prefix
        // We need to find the longest matching prefix first
        let mut best_match: Option<(&dyn LauncherView, usize)> = None;

        for view in &self.views {
            let prefix = view.prefix();
            if is_prefix(query, prefix) {
                // Check if this is a better (longer) match
                if best_match.is_none() || prefix.len() > best_match.unwrap().1 {
                    best_match = Some((view.as_ref(), prefix.len()));
                }
            }
        }

        // Also check help view
        if is_prefix(query, self.help_view.prefix()) {
            let prefix_len = self.help_view.prefix().len();
            if best_match.is_none() || prefix_len > best_match.unwrap().1 {
                best_match = Some((&self.help_view, prefix_len));
            }
        }

        if let Some((view, prefix_len)) = best_match {
            let rest = query[prefix_len..].trim_start();
            return (Some(view), rest);
        }

        // Check if starts with a known prefix character but no matching prefix
        // In this case, show help view
        if let Some(first_char) = query.chars().next()
            && (self
                .views
                .iter()
                .any(|v| v.prefix().starts_with(first_char))
                || self.help_view.prefix().starts_with(first_char))
        {
            // Unknown special prefix - show help
            return (Some(&self.help_view), query);
        }

        // No prefix, use default view with full query as search
        (self.default_view(), query)
    }

    /// Get the default view.
    fn default_view(&self) -> Option<&dyn LauncherView> {
        self.views
            .iter()
            .find(|v| v.is_default())
            .map(|v| v.as_ref())
    }

    /// Get the current active view.
    fn current_view(&self) -> &dyn LauncherView {
        let (view, _) = self.parse_query();
        view.unwrap_or(&self.help_view)
    }

    /// Get the current view name for display.
    fn current_view_name(&self) -> &str {
        self.current_view().name()
    }

    /// Create view context for rendering.
    fn view_context(&self) -> ViewContext<'_> {
        let (_, search) = self.parse_query();
        ViewContext {
            query: search,
            selected_index: self.selected_index,
        }
    }

    fn handle_input(&mut self, input: ViewInput, cx: &mut App) -> bool {
        let vx = self.view_context();
        let view = self.current_view();
        let item_count = view.match_count(&vx, cx);

        match view.handle_input(&input, &vx, cx) {
            InputResult::Handled { query, close } => {
                // Update search query based on current view prefix
                let (matched_view, _) = self.parse_query();
                if let Some(v) = matched_view {
                    let prefix = v.prefix();
                    if query.is_empty() {
                        self.input.set_text(prefix.to_string());
                    } else {
                        self.input.set_text(format!("{} {}", prefix, query));
                    }
                } else {
                    self.input.set_text(query);
                }
                self.selected_index = 0;
                self.reset_scroll();
                close
            }
            InputResult::Unhandled => {
                // Default handling
                match input {
                    ViewInput::Char(c) => {
                        self.input.insert_str(&c);
                        self.selected_index = 0;
                        self.reset_scroll();
                    }
                    ViewInput::Backspace => {
                        self.input.backspace();
                        self.selected_index = 0;
                        self.reset_scroll();
                    }
                    ViewInput::Up => {
                        if item_count > 0 {
                            self.selected_index = if self.selected_index == 0 {
                                item_count - 1
                            } else {
                                self.selected_index - 1
                            };
                            self.scroll_to_selected(cx);
                        }
                    }
                    ViewInput::Down => {
                        if item_count > 0 {
                            self.selected_index = (self.selected_index + 1) % item_count;
                            self.scroll_to_selected(cx);
                        }
                    }
                    ViewInput::PageUp => {
                        if item_count > 0 {
                            self.selected_index =
                                self.selected_index.saturating_sub(ITEMS_PER_PAGE);
                            self.scroll_to_selected(cx);
                        }
                    }
                    ViewInput::PageDown => {
                        if item_count > 0 {
                            self.selected_index = (self.selected_index + ITEMS_PER_PAGE)
                                .min(item_count.saturating_sub(1));
                            self.scroll_to_selected(cx);
                        }
                    }
                    ViewInput::Enter => {
                        return self.execute_selected(cx);
                    }
                }
                false
            }
        }
    }

    fn delete_word_back(&mut self) {
        self.input.delete_word_back();
        self.selected_index = 0;
        self.reset_scroll();
    }

    fn execute_selected(&mut self, cx: &mut App) -> bool {
        let view = self.current_view();

        // Check if we're in help view and selected a command
        if std::ptr::eq(view, &self.help_view as &dyn LauncherView) {
            let vx = self.view_context();
            if let Some(prefix) = self
                .help_view
                .selected_prefix(self.selected_index, vx.query)
            {
                let target = self.views.iter().find(|v| v.prefix() == prefix);
                if let Some(target_view) = target {
                    self.input.set_text(format!("{} ", target_view.prefix()));
                    self.selected_index = 0;
                    self.reset_scroll();
                    return false;
                }
            }
        }

        let vx = self.view_context();
        view.on_select(self.selected_index, &vx, cx)
    }

    fn placeholder(&self) -> String {
        "Search apps or type @, $, !, ? for commands...".to_string()
    }
}

impl Focusable for Launcher {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Launcher {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Always keep the launcher focused
        if !self.focus_handle.is_focused(window) {
            self.focus_handle.focus(window, cx);
        }

        let theme = cx.theme();

        let view_name = self.current_view_name().to_string();
        let placeholder = self.placeholder();

        // Compute item count and clamp selected index before borrowing self
        {
            let vx = self.view_context();
            let item_count = self.current_view().match_count(&vx, cx);
            if self.selected_index >= item_count && item_count > 0 {
                self.selected_index = item_count - 1;
            }
        }

        // Now render with the clamped selection
        let vx = self.view_context();
        let current_view = self.current_view();
        let item_count = current_view.match_count(&vx, cx);
        let footer_bar = current_view.render_footer_bar(&vx, cx);
        let selected_index = self.selected_index;
        let header = current_view.render_header(&vx, cx);
        let footer = current_view.render_footer(&vx, cx);
        let content = current_view.render_content(&vx, cx);

        // Pre-compute colors for closures
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let text_secondary = theme.text.secondary;
        let text_disabled = theme.text.disabled;
        let bg_primary = theme.bg.primary;
        let border_default = theme.border.default;
        let interactive_default = theme.interactive.default;

        div()
            .id("launcher")
            .track_focus(&self.focus_handle)
            .key_context("Launcher")
            .on_action(cx.listener(|this, _: &Cancel, window, cx| {
                if this.input.is_empty() {
                    // Clear the static handle before removing window
                    *LAUNCHER_WINDOW.lock().unwrap() = None;
                    window.remove_window();
                } else {
                    // First Esc clears input; second Esc closes.
                    this.input.clear();
                    this.selected_index = 0;
                    this.reset_scroll();
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &Confirm, window, cx| {
                if this.handle_input(ViewInput::Enter, cx) {
                    // Clear the static handle before removing window
                    *LAUNCHER_WINDOW.lock().unwrap() = None;
                    window.remove_window();
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CursorUp, _window, cx| {
                this.handle_input(ViewInput::Up, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CursorDown, _window, cx| {
                this.handle_input(ViewInput::Down, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &PageUp, _window, cx| {
                this.handle_input(ViewInput::PageUp, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &PageDown, _window, cx| {
                this.handle_input(ViewInput::PageDown, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &Backspace, _window, cx| {
                this.handle_input(ViewInput::Backspace, cx);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &DeleteWordBack, _window, cx| {
                this.delete_word_back();
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CursorLeft, _window, cx| {
                this.input.move_left(false);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CursorRight, _window, cx| {
                this.input.move_right(false);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &WordLeft, _window, cx| {
                this.input.move_word_left(false);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &WordRight, _window, cx| {
                this.input.move_word_right(false);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectWordLeft, _window, cx| {
                this.input.move_word_left(true);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectWordRight, _window, cx| {
                this.input.move_word_right(true);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectLeft, _window, cx| {
                this.input.move_left(true);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectRight, _window, cx| {
                this.input.move_right(true);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &SelectAll, _window, cx| {
                this.input.select_all();
                cx.notify();
            }))
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, _window, cx| {
                    if event.keystroke.modifiers.control || event.keystroke.modifiers.alt {
                        return;
                    }

                    let input_str = event.keystroke.key_char.as_ref().cloned().or_else(|| {
                        let key = event.keystroke.key.as_str();
                        if key.chars().count() == 1 {
                            Some(key.to_string())
                        } else {
                            None
                        }
                    });

                    let Some(s) = input_str else {
                        return;
                    };
                    if s.chars().any(|c| c.is_control()) {
                        return;
                    }

                    this.handle_input(ViewInput::Char(s), cx);
                    cx.notify();
                }),
            )
            .size_full()
            .bg(bg_primary)
            .border_1()
            .border_color(border_default)
            .rounded(px(radius::LG))
            .text_color(text_primary)
            .flex()
            .flex_col()
            .overflow_hidden()
            // Search input area
            .child(
                div()
                    .w_full()
                    .px(px(spacing::LG))
                    .py(px(spacing::MD))
                    .flex()
                    .items_center()
                    .gap(px(spacing::MD))
                    // Search icon
                    .child(
                        div()
                            .text_size(px(icon_size::LG))
                            .text_color(text_muted)
                            .child(""),
                    )
                    // Search text
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(font_size::MD))
                            .text_color(text_primary)
                            .child(render_input_line(&self.input, &placeholder, cx)),
                    )
                    // View badge (right side)
                    .child(
                        div()
                            .px(px(spacing::SM))
                            .py(px(3.))
                            .rounded(px(radius::SM))
                            .bg(interactive_default)
                            .text_size(px(font_size::SM))
                            .text_color(text_secondary)
                            .child(view_name),
                    ),
            )
            // Divider line
            .child(div().w_full().h(px(1.)).bg(border_default))
            // View content with scroll support — items are direct children
            // so that scroll_handle.scroll_to_item(index) works correctly.
            .child(
                div()
                    .id("view-content")
                    .flex_1()
                    .flex()
                    .flex_col()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .py(px(spacing::XS))
                    .when_some(header, |el, h| el.child(h))
                    .map(|el| {
                        if let Some(content) = content {
                            el.child(content)
                        } else {
                            el.children(
                                (0..item_count).map(|i| {
                                    current_view.render_item(i, i == selected_index, &vx, cx)
                                }),
                            )
                        }
                    })
                    .when_some(footer, |el, f| el.child(f)),
            )
            // Bottom footer bar
            .child(div().w_full().h(px(1.)).bg(border_default))
            .child(
                div()
                    .w_full()
                    .px(px(spacing::LG))
                    .py(px(spacing::SM))
                    .flex()
                    .items_center()
                    .justify_between()
                    // Left side - prefix hints
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::SM))
                            .text_size(px(font_size::XS))
                            .text_color(text_disabled)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .child("@apps")
                                    .child("·")
                                    .child("$shell")
                                    .child("·")
                                    .child("!web")
                                    .child("·")
                                    .child("?help"),
                            ),
                    )
                    // Right side - action hints from view
                    .child(footer_bar),
            )
    }
}

/// Global state to track the launcher window.
static LAUNCHER_WINDOW: std::sync::Mutex<Option<WindowHandle<Launcher>>> =
    std::sync::Mutex::new(None);

pub fn init(_cx: &mut App) {}

/// Toggle the launcher window with optional prefilled input.
///
/// Behavior:
/// - If launcher is closed: opens it (with optional input).
/// - If launcher is open and `input` is `Some`: updates the input.
/// - If launcher is open and `input` is `None`: closes it.
pub fn toggle(input: Option<String>, cx: &mut App) {
    let start = std::time::Instant::now();
    tracing::debug!("launcher::toggle: start");

    let mut guard = LAUNCHER_WINDOW.lock().unwrap();
    tracing::debug!("launcher::toggle: acquired lock {:?}", start.elapsed());

    if let Some(handle) = guard.take() {
        // If input is provided, update existing launcher instead of closing
        if let Some(input_text) = input {
            let update_result = handle.update(cx, |launcher, _, cx| {
                launcher.set_input(input_text);
                cx.notify();
            });
            if update_result.is_ok() {
                *guard = Some(handle);
                return;
            }
        }
        // No input or update failed, close the window
        let _ = handle.update(cx, |_, window, _| {
            window.remove_window();
        });
        tracing::debug!("launcher::toggle: closed window {:?}", start.elapsed());
    } else {
        tracing::debug!("launcher::toggle: opening new window {:?}", start.elapsed());
        let cfg = Config::global(cx).launcher.clone();
        if let Ok(handle) = cx.open_window(
            WindowOptions {
                titlebar: None,
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::new(px(0.), px(0.)),
                    size: Size::new(px(cfg.width), px(cfg.height)),
                })),
                app_id: Some("gpuishell-launcher".to_string()),
                window_background: WindowBackgroundAppearance::Transparent,
                kind: WindowKind::LayerShell(LayerShellOptions {
                    namespace: "launcher".to_string(),
                    layer: Layer::Overlay,
                    anchor: Anchor::TOP,
                    exclusive_zone: None,
                    margin: Some((
                        px(cfg.margin_top),
                        px(cfg.margin_right),
                        px(cfg.margin_bottom),
                        px(cfg.margin_left),
                    )),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    ..Default::default()
                }),
                focus: true,
                ..Default::default()
            },
            move |_, cx| {
                cx.new(|cx| {
                    let new_start = std::time::Instant::now();
                    let launcher = Launcher::new(input.clone(), cx);
                    tracing::debug!(
                        "launcher::toggle: Launcher::new took {:?}",
                        new_start.elapsed()
                    );
                    launcher
                })
            },
        ) {
            *guard = Some(handle);
            tracing::debug!("launcher::toggle: window opened {:?}", start.elapsed());
        }
    }
    tracing::debug!("launcher::toggle: done {:?}", start.elapsed());
}
